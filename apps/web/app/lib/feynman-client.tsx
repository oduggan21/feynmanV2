// --- Backend State ---
export interface SubTopic {
  name: string;
  has_definition: boolean;
  has_mechanism: boolean;
  has_example: boolean;
}

export interface FeynmanAgentState {
  main_topic: string;
  covered_subtopics: Record<string, SubTopic>;
  incomplete_subtopics: Record<string, SubTopic>;
}

export interface ChatMessage {
  id: number;
  session_id: string;
  role: "user" | "ai";
  content: string;
  created_at: string;
}

// --- WebSocket Protocol ---

// Messages sent FROM the browser client TO the server
type ClientToServerMessage =
  | {
      type: "init";
      topic: string;
      session_id?: string;
    }
  | {
      type: "user_message";
      text: string;
    }
  | {
      type: "set_voice_enabled";
      enabled: boolean;
    };

type ServerToClientMessage =
  | {
      type: "initialized";
      session_id: string;
      agent_state: FeynmanAgentState;
      history: ChatMessage[];
    }
  | { type: "response_start" }
  | { type: "response_chunk"; chunk: string }
  | { type: "response_end" }
  | { type: "state_update"; state: FeynmanAgentState }
  | { type: "error"; message: string }
  | { type: "transcription_update"; text: string; is_final: boolean }
  | { type: "audio_chunk"; data: string }
  | { type: "ai_speaking_start" }
  | { type: "ai_speaking_end" };

// --- Client-Side Events ---
interface FeynmanClientEvents {
  open: () => void;
  close: (event: CloseEvent) => void;
  error: (error: Event) => void;
  initialized: (data: {
    sessionId: string;
    agentState: FeynmanAgentState;
    history: ChatMessage[];
  }) => void;
  agentResponseStart: () => void;
  agentResponseChunk: (data: { chunk: string }) => void;
  agentResponseEnd: () => void;
  stateUpdate: (data: { state: FeynmanAgentState }) => void;
  serverError: (data: { message: string }) => void;
  transcriptionUpdate: (data: { text: string; isFinal: boolean }) => void;
  audioChunk: (data: { data: string }) => void;
  aiSpeakingStart: () => void;
  aiSpeakingEnd: () => void;
}

export class FeynmanClient {
  private ws: WebSocket | null = null;
  private listeners: {
    [K in keyof FeynmanClientEvents]?: Array<FeynmanClientEvents[K]>;
  } = {};

  constructor(private url: string) {}

  public on<K extends keyof FeynmanClientEvents>(
    event: K,
    listener: FeynmanClientEvents[K]
  ): void {
    if (!this.listeners[event]) {
      this.listeners[event] = [];
    }
    this.listeners[event]?.push(listener);
  }

  public off<K extends keyof FeynmanClientEvents>(
    event: K,
    listener: FeynmanClientEvents[K]
  ): void {
    const listenersForEvent = this.listeners[event];
    if (!listenersForEvent) return;
    const index = listenersForEvent.indexOf(listener);
    if (index > -1) {
      listenersForEvent.splice(index, 1);
    }
  }

  private emit<K extends keyof FeynmanClientEvents>(
    event: K,
    ...args: Parameters<FeynmanClientEvents[K]>
  ): void {
    this.listeners[event]?.forEach((listener) =>
      (listener as Function)(...args)
    );
  }

  public connect(topic: string, sessionId?: string): void {
    if (this.ws) {
      console.warn("FeynmanClient is already connected or connecting.");
      return;
    }

    console.log(`FeynmanClient: Attempting to connect to ${this.url}`);
    this.ws = new WebSocket(this.url);
    this.ws.binaryType = "arraybuffer";

    this.ws.onopen = () => {
      console.log(
        "FeynmanClient: WebSocket connection opened successfully (onopen event)."
      );
      this.sendMessageToServer({ type: "init", topic, session_id: sessionId });
      this.emit("open");
    };

    this.ws.onmessage = (event: MessageEvent) => {
      console.log(
        "FeynmanClient: Received raw message from server:",
        event.data
      );
      try {
        if (typeof event.data === "string") {
          const message: ServerToClientMessage = JSON.parse(event.data);
          this.handleServerMessage(message);
        }
      } catch (error) {
        console.error("FeynmanClient: Failed to parse server message.", error);
        console.error("Problematic data:", event.data);
      }
    };

    this.ws.onclose = (event: CloseEvent) => {
      console.log(
        `FeynmanClient: WebSocket connection closed (onclose event). Code: ${event.code}, Reason: ${event.reason}`
      );
      this.ws = null;
      this.emit("close", event);
    };

    this.ws.onerror = (error: Event) => {
      console.error("FeynmanClient: WebSocket error (onerror event).", error);
      this.emit("error", error);
    };
  }

  private handleServerMessage(message: ServerToClientMessage): void {
    switch (message.type) {
      case "initialized":
        this.emit("initialized", {
          sessionId: message.session_id,
          agentState: message.agent_state,
          history: message.history,
        });
        break;
      case "response_start":
        this.emit("agentResponseStart");
        break;
      case "response_chunk":
        this.emit("agentResponseChunk", { chunk: message.chunk });
        break;
      case "response_end":
        this.emit("agentResponseEnd");
        break;
      case "state_update":
        this.emit("stateUpdate", { state: message.state });
        break;
      case "error":
        this.emit("serverError", { message: message.message });
        break;
      case "transcription_update":
        this.emit("transcriptionUpdate", {
          text: message.text,
          isFinal: message.is_final,
        });
        break;
      case "audio_chunk":
        this.emit("audioChunk", { data: message.data });
        break;
      case "ai_speaking_start":
        this.emit("aiSpeakingStart");
        break;
      case "ai_speaking_end":
        this.emit("aiSpeakingEnd");
        break;
      default:
        console.warn(
          "FeynmanClient: Received unknown message type from server:",
          message
        );
    }
  }

  public sendUserMessage(text: string): void {
    this.sendMessageToServer({ type: "user_message", text });
  }

  public setVoiceEnabled(enabled: boolean): void {
    this.sendMessageToServer({ type: "set_voice_enabled", enabled });
  }

  public sendAudioChunk(chunk: ArrayBuffer): void {
    if (this.ws?.readyState === WebSocket.OPEN) {
      this.ws.send(chunk);
    }
  }

  public close(): void {
    if (this.ws) {
      this.ws.close();
    }
  }

  private sendMessageToServer(message: ClientToServerMessage): void {
    if (this.ws?.readyState !== WebSocket.OPEN) {
      console.error(
        "FeynmanClient: Cannot send message, WebSocket is not open."
      );
      return;
    }
    console.log("FeynmanClient: Sending message to server:", message);
    this.ws.send(JSON.stringify(message));
  }
}
