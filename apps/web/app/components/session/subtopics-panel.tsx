import {
  Accordion,
  AccordionContent,
  AccordionItem,
  AccordionTrigger,
} from "@revlentless/ui/components/accordion";
import { Badge } from "@revlentless/ui/components/badge";
import IconBadge from "~/components/ui/icon-badge";
import type { FeynmanAgentState } from "~/lib/feynman-client";
import { ScrollArea } from "@revlentless/ui/components/scroll-area";

export default function SubtopicsPanel({
  agentState,
}: {
  agentState: FeynmanAgentState | null;
}) {
  if (!agentState) return null;

  const allSubtopics = [
    ...Object.values(agentState.incomplete_subtopics),
    ...Object.values(agentState.covered_subtopics),
  ];

  return (
    <div className="flex h-full flex-col overflow-hidden">
      {" "}
      {/* Added overflow-hidden */}
      <div className="flex-shrink-0 border-b p-4">
        <h2 className="text-lg font-semibold">Curriculum</h2>
        <p className="text-sm text-muted-foreground">
          Your teaching plan for {agentState.main_topic}
        </p>
      </div>
      <ScrollArea className="flex-1 min-h-0">
        {" "}
        {/* Added min-h-0 */}
        <div className="p-4">
          <Accordion type="single" collapsible className="w-full">
            <AccordionItem value="legend">
              <AccordionTrigger className="text-sm">
                View Legend
              </AccordionTrigger>
              <AccordionContent>
                <div className="flex flex-wrap gap-2">
                  <Badge variant="outline" className="font-normal">
                    Pending
                  </Badge>
                  <Badge className="border-emerald-500/50 bg-emerald-500/10 text-emerald-700 dark:text-emerald-300">
                    Covered
                  </Badge>
                </div>
              </AccordionContent>
            </AccordionItem>
          </Accordion>
          <div className="mt-4 space-y-2">
            {allSubtopics.map((s) => (
              <div key={s.name} className="rounded-lg border p-3">
                <div className="font-medium">{s.name}</div>
                <div className="mt-2 flex flex-wrap gap-2">
                  <IconBadge
                    label="Definition"
                    state={s.has_definition ? "covered" : "pending"}
                  />
                  <IconBadge
                    label="Mechanism"
                    state={s.has_mechanism ? "covered" : "pending"}
                  />
                  <IconBadge
                    label="Example"
                    state={s.has_example ? "covered" : "pending"}
                  />
                </div>
              </div>
            ))}
          </div>
        </div>
      </ScrollArea>
    </div>
  );
}
