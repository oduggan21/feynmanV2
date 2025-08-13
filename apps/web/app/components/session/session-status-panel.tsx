import CircularProgress from "./circular-progress";
import { Brain, Mic, AudioLines, Timer } from "lucide-react";
import { cn } from "@revlentless/ui/lib/utils";
import { type AIStatus } from "~/providers/feynman-provider";
import { useEffect, useMemo, useState } from "react";
import type { FeynmanAgentState } from "~/lib/feynman-client";
import { ScrollArea } from "@revlentless/ui/components/scroll-area";
import { Separator } from "@revlentless/ui/components/separator";

function calculateProgress(agentState: FeynmanAgentState | null): number {
  if (!agentState) return 0;

  const allSubtopics = [
    ...Object.values(agentState.incomplete_subtopics),
    ...Object.values(agentState.covered_subtopics),
  ];

  if (allSubtopics.length === 0) return 0;

  let coveredCount = 0;
  const totalCriteria = allSubtopics.length * 3;

  for (const subtopic of allSubtopics) {
    if (subtopic.has_definition) coveredCount++;
    if (subtopic.has_mechanism) coveredCount++;
    if (subtopic.has_example) coveredCount++;
  }

  if (totalCriteria === 0) return 0;
  return Math.round((coveredCount / totalCriteria) * 100);
}

export default function SessionStatusPanel({
  aiStatus = "listening",
  agentState,
}: {
  aiStatus?: AIStatus;
  agentState: FeynmanAgentState | null;
}) {
  const [elapsedSec, setElapsedSec] = useState(0);
  const status = getStatus(aiStatus);
  const elapsed = formatTime(elapsedSec);
  const progress = useMemo(() => calculateProgress(agentState), [agentState]);

  useEffect(() => {
    const timer = setInterval(() => {
      setElapsedSec((s) => s + 1);
    }, 1000);
    return () => clearInterval(timer);
  }, []);

  return (
    <div className="flex h-full flex-col">
      <div className="flex-shrink-0 border-b p-4">
        <h2 className="text-lg font-semibold">Session Status</h2>
        <p className="text-sm text-muted-foreground">
          Live progress and AI state
        </p>
      </div>
      <ScrollArea className="flex-1">
        <div className="p-4">
          <div className="space-y-6">
            <div>
              <h3 className="mb-2 text-sm font-medium text-muted-foreground">
                Overall Progress
              </h3>
              <div className="flex items-center justify-center p-4">
                <CircularProgress
                  value={progress}
                  size={140}
                  strokeWidth={12}
                />
              </div>
            </div>

            <Separator />

            <div className="space-y-4">
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">AI Status</span>
                <div className="flex items-center gap-2">
                  <status.Icon className={cn("h-4 w-4", status.color)} />
                  <span className="font-medium">{status.title}</span>
                </div>
              </div>
              <div className="flex items-center justify-between">
                <span className="text-sm text-muted-foreground">
                  Session Time
                </span>
                <span className="font-mono font-medium">{elapsed}</span>
              </div>
            </div>
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}

function getStatus(s: AIStatus) {
  if (s === "thinking")
    return {
      title: "Thinking...",
      Icon: Brain,
      color: "text-purple-500",
    };
  if (s === "speaking")
    return {
      title: "Responding...",
      Icon: AudioLines,
      color: "text-amber-500",
    };
  return {
    title: "Listening",
    Icon: Mic,
    color: "text-emerald-500",
  };
}

function formatTime(sec: number) {
  const m = Math.floor(sec / 60);
  const s = sec % 60;
  return `${String(m).padStart(2, "0")}:${String(s).padStart(2, "0")}`;
}
