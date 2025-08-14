import { useNavigate, useParams } from "react-router";
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
  AlertDialogTrigger,
} from "@revlentless/ui/components/alert-dialog";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@revlentless/ui/components/dialog";
import { Button } from "@revlentless/ui/components/button";
import {
  Sheet,
  SheetContent,
  SheetTrigger,
} from "@revlentless/ui/components/sheet";
import { useFeynman } from "~/providers/feynman-provider";
import { useEffect, useMemo, useState } from "react";
import {
  Loader2,
  PartyPopper,
  PanelLeftClose,
  PanelLeftOpen,
  PanelRightClose,
  PanelRightOpen,
  X,
} from "lucide-react";
import SubtopicsPanel from "~/components/session/subtopics-panel";
import ConversationView from "~/components/session/conversation-view";
import SessionStatusPanel from "~/components/session/session-status-panel";
import { cn } from "@revlentless/ui/lib/utils";
import { ThemeToggle } from "@revlentless/ui-theme/components/theme-toggle";
import { ThemeSelector } from "@revlentless/ui-theme/components/theme-selector";

export default function SessionDetail() {
  const { id: sessionIdFromUrl } = useParams();
  const navigate = useNavigate();
  const { connect, disconnect, isConnected, agentState, messages, aiStatus } =
    useFeynman();

  const [isLeftPanelOpen, setIsLeftPanelOpen] = useState(true);
  const [isRightPanelOpen, setIsRightPanelOpen] = useState(true);

  useEffect(() => {
    if (!sessionIdFromUrl) return;
    connect("Resuming Session", sessionIdFromUrl);
    return () => disconnect();
  }, [sessionIdFromUrl, connect, disconnect]);

  const isSessionComplete = useMemo(() => {
    if (!agentState) return false;
    return (
      Object.keys(agentState.incomplete_subtopics).length === 0 &&
      Object.keys(agentState.covered_subtopics).length > 0
    );
  }, [agentState]);

  const handleEndSession = () => {
    disconnect();
    navigate("/sessions");
  };

  if (!isConnected) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Connecting to session...
        </div>
      </div>
    );
  }

  if (!agentState) {
    return (
      <div className="flex h-screen w-full items-center justify-center bg-background">
        <div className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          Initializing session state...
        </div>
      </div>
    );
  }

  return (
    <div className="flex h-screen w-full flex-col bg-background">
      {/* HEADER */}
      <header className="flex h-14 items-center justify-between border-b bg-background px-4">
        <div className="flex items-center gap-2">
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setIsLeftPanelOpen(!isLeftPanelOpen)}
            className="hidden lg:flex"
          >
            {isLeftPanelOpen ? (
              <PanelLeftClose className="h-5 w-5" />
            ) : (
              <PanelLeftOpen className="h-5 w-5" />
            )}
          </Button>
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon" className="lg:hidden">
                <PanelLeftOpen className="h-5 w-5" />
              </Button>
            </SheetTrigger>
            <SheetContent side="left" className="w-[300px] p-0">
              <SubtopicsPanel agentState={agentState} />
            </SheetContent>
          </Sheet>
          <h1 className="text-lg font-semibold">{agentState.main_topic}</h1>
        </div>

        <div className="flex flex-row gap-x-6">
          <ThemeToggle />
          <ThemeSelector />
        </div>
        <div className="flex items-center gap-2">
          <AlertDialog>
            <AlertDialogTrigger asChild>
              <Button size="sm" variant="destructive">
                <X className="mr-2 h-4 w-4" /> End Session
              </Button>
            </AlertDialogTrigger>
            <AlertDialogContent>
              <AlertDialogHeader>
                <AlertDialogTitle>End session?</AlertDialogTitle>
                <AlertDialogDescription>
                  This will disconnect you from the agent. You can resume this
                  session later.
                </AlertDialogDescription>
              </AlertDialogHeader>
              <AlertDialogFooter>
                <AlertDialogCancel>Cancel</AlertDialogCancel>
                <AlertDialogAction
                  className="bg-destructive hover:bg-destructive/90"
                  onClick={handleEndSession}
                >
                  End session
                </AlertDialogAction>
              </AlertDialogFooter>
            </AlertDialogContent>
          </AlertDialog>
          <Sheet>
            <SheetTrigger asChild>
              <Button variant="ghost" size="icon" className="lg:hidden">
                <PanelRightOpen className="h-5 w-5" />
              </Button>
            </SheetTrigger>
            <SheetContent side="right" className="w-[300px] p-0">
              <SessionStatusPanel agentState={agentState} aiStatus={aiStatus} />
            </SheetContent>
          </Sheet>
          <Button
            variant="ghost"
            size="icon"
            onClick={() => setIsRightPanelOpen(!isRightPanelOpen)}
            className="hidden lg:flex"
          >
            {isRightPanelOpen ? (
              <PanelRightClose className="h-5 w-5" />
            ) : (
              <PanelRightOpen className="h-5 w-5" />
            )}
          </Button>
        </div>
      </header>

      {/* MAIN CONTENT AREA */}
      <main className="flex flex-1 overflow-hidden">
        {/* Left Sidebar (Curriculum) */}
        <aside
          className={cn(
            "hidden lg:flex flex-col border-r bg-muted/20 transition-all duration-300",
            isLeftPanelOpen ? "w-[320px]" : "w-0 overflow-hidden"
          )}
        >
          <SubtopicsPanel agentState={agentState} />
        </aside>

        {/* Center (Conversation) */}
        <div className="flex-1">
          <ConversationView messages={messages} aiStatus={aiStatus} />
        </div>

        {/* Right Sidebar (Status) */}
        <aside
          className={cn(
            "hidden lg:flex flex-col border-l bg-muted/20 transition-all duration-300",
            isRightPanelOpen ? "w-[320px]" : "w-0 overflow-hidden"
          )}
        >
          <SessionStatusPanel agentState={agentState} aiStatus={aiStatus} />
        </aside>
      </main>

      {/* Session Completion Dialog */}
      <Dialog open={isSessionComplete}>
        <DialogContent>
          <DialogHeader>
            <DialogTitle className="flex items-center gap-2">
              <PartyPopper className="h-6 w-6 text-amber-500" />
              Congratulations!
            </DialogTitle>
            <DialogDescription>
              You've successfully taught all subtopics for "
              {agentState.main_topic}". You have mastered the material!
            </DialogDescription>
          </DialogHeader>
          <DialogFooter>
            <Button onClick={handleEndSession} className="w-full">
              Finish and Return to Dashboard
            </Button>
          </DialogFooter>
        </DialogContent>
      </Dialog>
    </div>
  );
}
