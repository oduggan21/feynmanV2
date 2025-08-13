// app/components/session/conversation-view.tsx

import { useEffect, useRef, useState, type FormEvent } from "react";
import { Avatar, AvatarFallback } from "@revlentless/ui/components/avatar";
import { Bot, Mic, MicOff, Send, User } from "lucide-react";
import { Button } from "@revlentless/ui/components/button";
import { Textarea } from "@revlentless/ui/components/textarea";
import { useFeynman, type AIStatus } from "~/providers/feynman-provider";
import type { ChatMessage } from "~/lib/feynman-client";
import { cn } from "@revlentless/ui/lib/utils";
import { ScrollArea } from "@revlentless/ui/components/scroll-area";

export default function ConversationView({
  messages = [],
  aiStatus,
}: {
  messages?: ChatMessage[];
  aiStatus: AIStatus;
}) {
  const scrollRef = useRef<HTMLDivElement>(null);
  const {
    sendUserMessage,
    liveTranscript,
    isRecording,
    startRecording,
    stopRecording,
  } = useFeynman();
  const [inputText, setInputText] = useState("");

  useEffect(() => {
    const el = scrollRef.current;
    if (el) {
      el.scrollTop = el.scrollHeight;
    }
  }, [messages, liveTranscript]);

  const handleSubmit = (e: FormEvent) => {
    e.preventDefault();
    const text = inputText.trim();
    if (text) {
      sendUserMessage(text);
      setInputText("");
    }
  };

  const canInteract = aiStatus === "listening";

  return (
    <div className="flex h-full flex-col">
      <ScrollArea className="flex-1 h-full" ref={scrollRef}>
        <div className="mx-auto max-w-3xl space-y-8 p-4 lg:p-8">
          {messages.map((m, i) => (
            <div
              key={m.id || `msg-${i}`}
              className={cn(
                "flex items-start gap-4",
                m.role === "user" && "justify-end"
              )}
            >
              {m.role === "ai" && (
                <Avatar className="h-8 w-8 border">
                  <AvatarFallback aria-label="AI">
                    <Bot className="h-4 w-4" />
                  </AvatarFallback>
                </Avatar>
              )}
              <div
                className={cn(
                  "max-w-[85%] rounded-lg px-4 py-3 text-sm",
                  m.role === "ai"
                    ? "bg-muted"
                    : "bg-primary text-primary-foreground"
                )}
              >
                <div className="prose prose-sm dark:prose-invert max-w-none break-words">
                  {m.content}
                </div>
              </div>
              {m.role === "user" && (
                <Avatar className="h-8 w-8 border">
                  <AvatarFallback
                    aria-label="User"
                    className="bg-primary/10 text-primary"
                  >
                    <User className="h-4 w-4" />
                  </AvatarFallback>
                </Avatar>
              )}
            </div>
          ))}

          {liveTranscript && (
            <div className="flex items-start gap-4 justify-end">
              <div className="max-w-[85%] rounded-lg px-4 py-3 text-sm bg-primary/80 text-primary-foreground opacity-70">
                <div className="prose prose-sm dark:prose-invert max-w-none break-words">
                  {liveTranscript}
                </div>
              </div>
              <Avatar className="h-8 w-8 border">
                <AvatarFallback
                  aria-label="User"
                  className="bg-primary/10 text-primary"
                >
                  <User className="h-4 w-4" />
                </AvatarFallback>
              </Avatar>
            </div>
          )}

          {aiStatus === "thinking" && (
            <div className="flex items-start gap-4">
              <Avatar className="h-8 w-8 border">
                <AvatarFallback aria-label="AI">
                  <Bot className="h-4 w-4" />
                </AvatarFallback>
              </Avatar>
              <div className="rounded-lg bg-muted px-4 py-3">
                <div className="flex items-center gap-2">
                  <span className="h-2 w-2 animate-pulse rounded-full bg-foreground/50" />
                  <span className="h-2 w-2 animate-pulse rounded-full bg-foreground/50 [animation-delay:0.2s]" />
                  <span className="h-2 w-2 animate-pulse rounded-full bg-foreground/50 [animation-delay:0.4s]" />
                </div>
              </div>
            </div>
          )}
        </div>
      </ScrollArea>

      <div className="border-t bg-background/50 px-4 py-3 backdrop-blur-sm">
        <form
          onSubmit={handleSubmit}
          className="relative mx-auto max-w-3xl flex items-center gap-2"
        >
          <Button
            type="button"
            variant={isRecording ? "destructive" : "outline"}
            size="icon"
            onClick={() => (isRecording ? stopRecording() : startRecording())}
            disabled={!canInteract}
          >
            {isRecording ? (
              <MicOff className="h-4 w-4" />
            ) : (
              <Mic className="h-4 w-4" />
            )}
            <span className="sr-only">
              {isRecording ? "Stop recording" : "Start recording"}
            </span>
          </Button>
          <Textarea
            placeholder={
              isRecording
                ? "Listening..."
                : "Explain the next concept, or ask a clarifying question..."
            }
            className="flex-1 min-h-[52px] resize-none rounded-xl border-2 p-3 pr-14"
            value={inputText}
            onChange={(e) => setInputText(e.target.value)}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSubmit(e);
              }
            }}
            disabled={!canInteract || isRecording}
          />
          <Button
            type="submit"
            size="icon"
            className="absolute right-3 top-1/2 -translate-y-1/2 rounded-lg"
            disabled={!canInteract || !inputText.trim() || isRecording}
          >
            <Send className="h-4 w-4" />
            <span className="sr-only">Send</span>
          </Button>
        </form>
      </div>
    </div>
  );
}
