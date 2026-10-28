import { notFound } from "next/navigation";
import { AgentConsole, type Section } from "@/components/console";

const sections: Section[] = ["agents", "economy", "memory", "social", "trading", "launch", "factory"];
export default async function SectionPage({ params }: { params: Promise<{ section: string }> }) {
  const { section } = await params;
  if (!sections.includes(section as Section)) notFound();
  return <AgentConsole section={section as Section}/>;
}
