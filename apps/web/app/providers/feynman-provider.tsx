import {
  createContext,
  useContext,
  useCallback,
  useMemo,
  useRef,
  useState,
  type ReactNode,
  useEffect,
} from "react";
import { useQueryClient } from "@tanstack/react-query";
import {
  useCreateSession,
  getListSessionsQueryKey,
} from "@workspace/feynman-query";
import {
  FeynmanClient,
  type FeynmanAgentState,
  type ChatMessage,
} from "~/lib/feynman-client";
import { toast } from "@revlentless/ui/components/sonner";
import { axios } from "~/lib/axios";
import { useVoiceRecorder } from "~/hooks/use-voice-recorder";
import { AudioPlayer } from "~/lib/audio-player";

export type AIStatus = "listening" | "thinking" | "speaking";

type FeynmanContextType = {
  isConnected: boolean;
  aiStatus: AIStatus;
  sessionId: string | null;
  agentState: FeynmanAgentState | null;
  messages: ChatMessage[];
  liveTranscript: string;
  connect: (topic: string, sessionId?: string) => void;
  disconnect: () => void;
  sendUserMessage: (text: string) => void;
  createSession: (
    topic: string,
    options?: {
      onSuccess?: (sessionId: string) => void;
      onError?: (error: Error) => void;
    }
  ) => void;
  isCreatingSession: boolean;
  isRecording: boolean;
  startRecording: () => void;
  stopRecording: () => void;
};

const FeynmanContext = createContext<FeynmanContextType | null>(null);

export function FeynmanProvider({ children }: { children: ReactNode }) {
  const queryClient = useQueryClient();

  const [isConnected, setIsConnected] = useState(false);
  const [aiStatus, setAiStatus] = useState<AIStatus>("listening");
  const [sessionId, setSessionId] = useState<string | null>(null);
  const [agentState, setAgentState] = useState<FeynmanAgentState | null>(null);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [liveTranscript, setLiveTranscript] = useState("");
  const clientRef = useRef<FeynmanClient | null>(null);
  const audioPlayerRef = useRef<AudioPlayer | null>(null);

  const isConnectedRef = useRef(isConnected);
  const sessionIdRef = useRef(sessionId);
  useEffect(() => {
    isConnectedRef.current = isConnected;
    sessionIdRef.current = sessionId;
  }, [isConnected, sessionId]);

  const { mutate: createSessionMutation, isPending: isCreatingSession } =
    useCreateSession({
      axios,
      mutation: {
        onSuccess: (response) => {
          const newSession = response.data;
          toast.success("Session created", {
            description: `Topic: ${newSession.topic}`,
          });
          queryClient.invalidateQueries({
            queryKey: getListSessionsQueryKey(),
          });
        },
        onError: (error) => {
          toast.error("Failed to create session", {
            description: error.response?.data?.message || error.message,
          });
        },
      },
    });

  const sendAudioData = useCallback((data: ArrayBuffer) => {
    clientRef.current?.sendAudioChunk(data);
  }, []);

  const {
    start: startRecorderHook,
    stop: stopRecorderHook,
    isRecording,
  } = useVoiceRecorder(sendAudioData);

  const isRecordingRef = useRef(isRecording);
  useEffect(() => {
    isRecordingRef.current = isRecording;
  }, [isRecording]);

  const createSession = useCallback(
    (
      topic: string,
      options?: {
        onSuccess?: (sessionId: string) => void;
        onError?: (error: Error) => void;
      }
    ) => {
      createSessionMutation(
        { data: { topic } },
        {
          onSuccess: (response) => options?.onSuccess?.(response.data.id),
          onError: (error) => options?.onError?.(error),
        }
      );
    },
    [createSessionMutation]
  );

  const disconnect = useCallback(() => {
    if (isRecordingRef.current) {
      stopRecorderHook();
    }
    clientRef.current?.close();
    clientRef.current = null;
    audioPlayerRef.current?.stop();
    audioPlayerRef.current = null;
  }, [stopRecorderHook]);

  const connect = useCallback((topic: string, sessionIdToResume?: string) => {
    if (clientRef.current) return;

    const wsUrl = import.meta.env.VITE_WS_URL || "ws://localhost:3000/ws";
    const client = new FeynmanClient(wsUrl);
    clientRef.current = client;
    audioPlayerRef.current = new AudioPlayer();

    client.on("open", () => setIsConnected(true));

    client.on("initialized", (data) => {
      setSessionId(data.sessionId);
      setAgentState(data.agentState);
      setMessages(data.history);
      setAiStatus("listening");
      toast.info(`Session started for topic: ${data.agentState.main_topic}`);
    });

    client.on("agentResponseStart", () => {
      setAiStatus("speaking");
      setMessages((prev) => [
        ...prev,
        {
          id: Date.now(),
          session_id: sessionIdToResume || "",
          role: "ai",
          content: "",
          created_at: new Date().toISOString(),
        },
      ]);
    });

    client.on("agentResponseChunk", (data) => {
      setMessages((prev) => {
        const newMessages = [...prev];
        const lastMessage = newMessages[newMessages.length - 1];
        if (lastMessage?.role === "ai") {
          lastMessage.content += data.chunk;
        }
        return newMessages;
      });
    });

    client.on("agentResponseEnd", () => setAiStatus("listening"));
    client.on("aiSpeakingStart", () => setAiStatus("speaking"));
    client.on("aiSpeakingEnd", () => setAiStatus("listening"));
    client.on("audioChunk", ({ data }) =>
      audioPlayerRef.current?.addChunk(data)
    );
    client.on("transcriptionUpdate", ({ text, isFinal }) => {
      setLiveTranscript(text);
      if (isFinal) setLiveTranscript("");
    });
    client.on("stateUpdate", (data) => {
      setAgentState(data.state);
      toast.success("Progress updated!");
    });
    client.on("serverError", (error) => {
      toast.error("Server Error", { description: error.message });
      setAiStatus("listening");
    });
    client.on("close", () => {
      setIsConnected(false);
      setSessionId(null);
      setAgentState(null);
      setMessages([]);
      audioPlayerRef.current?.stop();
      toast.warning("Disconnected from agent.");
    });

    client.connect(topic, sessionIdToResume);
  }, []);

  const sendUserMessage = useCallback((text: string) => {
    if (!isConnectedRef.current || !sessionIdRef.current) {
      toast.error("Not connected to the agent.");
      return;
    }
    clientRef.current?.sendUserMessage(text);
    setMessages((prev) => [
      ...prev,
      {
        id: Date.now(),
        session_id: sessionIdRef.current || "",
        role: "user",
        content: text,
        created_at: new Date().toISOString(),
      },
    ]);
    setAiStatus("thinking");
  }, []);

  const startRecording = useCallback(() => {
    if (!isConnectedRef.current) {
      toast.error("Not connected to agent.");
      return;
    }
    clientRef.current?.setVoiceEnabled(true);
    startRecorderHook();
  }, [startRecorderHook]);

  const stopRecording = useCallback(() => {
    if (!isConnectedRef.current) {
      return;
    }
    stopRecorderHook();
    clientRef.current?.setVoiceEnabled(false);
  }, [stopRecorderHook]);
  const value = useMemo(
    () => ({
      isConnected,
      aiStatus,
      sessionId,
      agentState,
      messages,
      liveTranscript,
      connect,
      disconnect,
      sendUserMessage,
      createSession,
      isCreatingSession,
      isRecording,
      startRecording,
      stopRecording,
    }),
    [
      isConnected,
      aiStatus,
      sessionId,
      agentState,
      messages,
      liveTranscript,
      connect,
      disconnect,
      sendUserMessage,
      createSession,
      isCreatingSession,
      isRecording,
      startRecording,
      stopRecording,
    ]
  );

  return (
    <FeynmanContext.Provider value={value}>{children}</FeynmanContext.Provider>
  );
}

export function useFeynman() {
  const context = useContext(FeynmanContext);
  if (!context) {
    throw new Error("useFeynman must be used within a FeynmanProvider");
  }
  return context;
}
