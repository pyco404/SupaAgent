import { z } from "zod";
import { notFound } from "next/navigation";
import { AgentConsole, type Section } from "@/components/console";

const sections: Section[] = ["agents", "economy", "memory", "social", "trading", "launch", "factory"];
export default async function SectionPage({ params, searchParams }: { params: Promise<{ section: string }>; searchParams: Promise<{ agent?: string }> }) {
  const { section } = await params;
  if (!sections.includes(section as Section)) notFound();
  const requested = (await searchParams).agent;
  const initialAgentId = z.uuid().safeParse(requested).success ? requested : undefined;
  return <AgentConsole key={`${section}:${initialAgentId ?? ""}`} section={section as Section} initialAgentId={initialAgentId}/>;
}
