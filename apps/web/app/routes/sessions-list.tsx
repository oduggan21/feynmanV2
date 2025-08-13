import { Link } from "react-router";
import { Button } from "@revlentless/ui/components/button";
import { Badge } from "@revlentless/ui/components/badge";
import { Input } from "@revlentless/ui/components/input";
import {
  Tabs,
  TabsContent,
  TabsList,
  TabsTrigger,
} from "@revlentless/ui/components/tabs";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@revlentless/ui/components/table";
import {
  Card,
  CardContent,
  CardHeader,
  CardTitle,
} from "@revlentless/ui/components/card";
import { useListSessions } from "@workspace/feynman-query";
import { ArrowRight, Filter, Plus, Loader2 } from "lucide-react";
import { Progress } from "@revlentless/ui/components/progress";
import { useMemo, useState } from "react";
import type { Session } from "@workspace/feynman-query/schemas";

export default function SessionsList() {
  const { data: response, isLoading, isError } = useListSessions();

  // FIX: safely ensure sessions is always an array
  const sessions: Session[] = Array.isArray(response?.data)
    ? response.data
    : [];

  const [query, setQuery] = useState("");

  const filtered = useMemo(
    () =>
      sessions.filter(
        (s) =>
          s.topic.toLowerCase().includes(query.toLowerCase()) ||
          s.id.toLowerCase().includes(query.toLowerCase())
      ),
    [sessions, query]
  );

  const active = filtered.filter((s) => s.status === "active");
  const ended = filtered.filter((s) => s.status === "ended");

  return (
    <div className="space-y-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-2xl font-bold tracking-tight">Sessions</h1>
          <p className="text-muted-foreground">
            Browse and manage your teaching sessions.
          </p>
        </div>
        <Button asChild className="bg-primary hover:bg-emerald-700">
          <Link to="/sessions/new?topic=Data%20Structures">
            <Plus className="mr-2 h-4 w-4" />
            New Session
          </Link>
        </Button>
      </div>

      <div className="flex items-center gap-2">
        <div className="relative flex-1">
          <Input
            placeholder="Search by topic or session id..."
            value={query}
            onChange={(e) => setQuery(e.target.value)}
          />
          <Filter className="pointer-events-none absolute right-3 top-1/2 h-4 w-4 -translate-y-1/2 text-muted-foreground" />
        </div>
      </div>

      <Tabs defaultValue="active">
        <TabsList>
          <TabsTrigger value="active">Active</TabsTrigger>
          <TabsTrigger value="ended">Ended</TabsTrigger>
          <TabsTrigger value="all">All</TabsTrigger>
        </TabsList>

        <TabsContent value="active" className="mt-4">
          <SessionTable rows={active} isLoading={isLoading} isError={isError} />
        </TabsContent>
        <TabsContent value="ended" className="mt-4">
          <SessionTable rows={ended} isLoading={isLoading} isError={isError} />
        </TabsContent>
        <TabsContent value="all" className="mt-4">
          <SessionTable
            rows={filtered}
            isLoading={isLoading}
            isError={isError}
          />
        </TabsContent>
      </Tabs>
    </div>
  );
}

function SessionTable({
  rows,
  isLoading,
  isError,
}: {
  rows: Session[];
  isLoading: boolean;
  isError: boolean;
}) {
  return (
    <Card>
      <CardHeader>
        <CardTitle className="text-base">Sessions ({rows.length})</CardTitle>
      </CardHeader>
      <CardContent className="overflow-x-auto">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>Topic</TableHead>
              <TableHead className="hidden sm:table-cell">Session ID</TableHead>
              <TableHead>Status</TableHead>
              <TableHead>Progress</TableHead>
              <TableHead className="text-right">Open</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {isLoading ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center">
                  <div className="flex items-center justify-center gap-2 text-muted-foreground">
                    <Loader2 className="h-4 w-4 animate-spin" />
                    Loading sessions...
                  </div>
                </TableCell>
              </TableRow>
            ) : isError ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center text-red-600">
                  Failed to load sessions.
                </TableCell>
              </TableRow>
            ) : rows.length === 0 ? (
              <TableRow>
                <TableCell
                  colSpan={5}
                  className="text-center text-muted-foreground"
                >
                  No sessions found.
                </TableCell>
              </TableRow>
            ) : (
              rows.map((s) => {
                const pct = 0; // Progress is handled elsewhere
                return (
                  <TableRow key={s.id}>
                    <TableCell className="font-medium">{s.topic}</TableCell>
                    <TableCell className="hidden sm:table-cell font-mono text-xs text-muted-foreground">
                      {s.id}
                    </TableCell>
                    <TableCell>
                      <Badge
                        variant={
                          s.status === "active" ? "default" : "secondary"
                        }
                      >
                        {s.status}
                      </Badge>
                    </TableCell>
                    <TableCell>
                      <div className="flex items-center gap-2">
                        <Progress value={pct} className="h-2 w-24" />
                        <span className="text-xs text-muted-foreground">
                          {pct}%
                        </span>
                      </div>
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
                );
              })
            )}
          </TableBody>
        </Table>
      </CardContent>
    </Card>
  );
}
