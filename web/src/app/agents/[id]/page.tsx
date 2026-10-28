import { notFound } from "next/navigation";
import { z } from "zod";
import { AgentConsole } from "@/components/console";

export default async function ProfilePage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  if (!z.uuid().safeParse(id).success) notFound();
  return <AgentConsole section="profile" agentId={id}/>;
}
