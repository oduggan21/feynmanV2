import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { Badge } from "@revlentless/ui/components/badge";
import { Button } from "@revlentless/ui/components/button";
import { ArrowRight, Clock, BookOpen, Sparkles } from "lucide-react";
import { Link } from "react-router";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@revlentless/ui/components/table";
import { useListSessions } from "@workspace/feynman-query";
import type { Session } from "@workspace/feynman-query/schemas";

export default function Dashboard() {
  const { data: response, isLoading } = useListSessions();
  const sessions: Session[] = Array.isArray(response?.data)
    ? response.data
    : [];

  const totalSessions = sessions.length;
  const activeSessions = sessions?.filter((s) => s.status === "active").length;

  const recent = sessions.slice(0, 5);

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Dashboard</h1>
          <p className="text-muted-foreground">
            A quick overview of your teaching sessions.
          </p>
        </div>
        <Button asChild className="bg-primary hover:bg-emerald-700">
          <Link to="/sessions/new?topic=Operating%20Systems">
            <Sparkles className="mr-2 h-4 w-4" />
            New Session
          </Link>
        </Button>
      </div>

      <div className="grid gap-4 md:grid-cols-3">
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Total Sessions
            </CardTitle>
            <BookOpen className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoading ? "..." : totalSessions}
            </div>
            <p className="text-xs text-muted-foreground">
              All-time sessions from database
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader className="flex flex-row items-center justify-between space-y-0 pb-2">
            <CardTitle className="text-sm font-medium">
              Active Sessions
            </CardTitle>
            <Clock className="h-4 w-4 text-muted-foreground" />
          </CardHeader>
          <CardContent>
            <div className="text-2xl font-bold">
              {isLoading ? "..." : activeSessions}
            </div>
            <p className="text-xs text-muted-foreground">
              Currently in progress
            </p>
          </CardContent>
        </Card>
        <Card>
          <CardHeader>
            <CardTitle>Getting Started</CardTitle>
            <CardDescription>
              Explain a topic out loud to learn it better.
            </CardDescription>
          </CardHeader>
          <CardContent>
            <p className="text-sm text-muted-foreground">
              Click "New Session" to start. Your AI student will ask questions
              to help you find gaps in your understanding.
            </p>
          </CardContent>
        </Card>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>Recent Sessions</CardTitle>
          <CardDescription>Last 5 sessions you started</CardDescription>
        </CardHeader>
        <CardContent className="overflow-x-auto">
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>Topic</TableHead>
                <TableHead>Status</TableHead>
                <TableHead>Created At</TableHead>
                <TableHead className="text-right">Action</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {isLoading ? (
                <TableRow>
                  <TableCell colSpan={4} className="text-center">
                    Loading...
                  </TableCell>
                </TableRow>
              ) : recent.length === 0 ? (
                <TableRow>
                  <TableCell
                    colSpan={4}
                    className="text-center text-muted-foreground"
                  >
                    No sessions yet. Start one!
                  </TableCell>
                </TableRow>
              ) : (
                recent.map((s) => (
                  <TableRow key={s.id}>
                    <TableCell className="font-medium">{s.topic}</TableCell>
                    <TableCell>
                      <Badge
                        variant={
                          s.status === "active" ? "default" : "secondary"
                        }
                      >
                        {s.status}
                      </Badge>
                    </TableCell>
                    <TableCell className="text-muted-foreground text-sm">
                      {new Date(s.created_at).toLocaleString()}
                    </TableCell>
                    <TableCell className="text-right">
                      <Button asChild variant="outline" size="sm">
                        <Link to={`/sessions/${s.id}`}>
                          Open
                          <ArrowRight className="ml-2 h-4 w-4" />
                        </Link>
                      </Button>
                    </TableCell>
                  </TableRow>
                ))
              )}
            </TableBody>
          </Table>
        </CardContent>
      </Card>
    </div>
  );
}
