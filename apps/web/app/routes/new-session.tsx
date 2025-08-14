import { useEffect } from "react";
import { useNavigate, useSearchParams } from "react-router";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Loader2 } from "lucide-react";
import { useFeynman } from "~/providers/feynman-provider";

export default function NewSession() {
  const { createSession, isCreatingSession } = useFeynman();
  const [params] = useSearchParams();
  const navigate = useNavigate();
  const topic = params.get("topic") || "Untitled Topic";

  useEffect(() => {
    // On success, we get the real session ID from the database and navigate.
    createSession(topic, {
      onSuccess: (sessionId) => {
        const to = `/sessions/${sessionId}?topic=${encodeURIComponent(topic)}`;
        navigate(to, { replace: true });
      },
      onError: () => {
        navigate("/dashboard");
      },
    });
  }, [topic, createSession, navigate]);

  return (
    <div className="flex min-h-[50vh] items-center justify-center">
      <Card>
        <CardHeader>
          <CardTitle>Generating Curriculum...</CardTitle>
          <CardDescription>
            Please wait while we prepare a learning plan for "{topic}".
          </CardDescription>
        </CardHeader>
        <CardContent className="flex items-center gap-2 text-muted-foreground">
          <Loader2 className="h-4 w-4 animate-spin" />
          This may take a few moments.
        </CardContent>
      </Card>
    </div>
  );
}
