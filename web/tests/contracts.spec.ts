import { expect, test } from "@playwright/test";
import { schemas, units } from "../src/lib/contracts";
import { agentId, agent, financials } from "./fixtures";

test("financial contracts reject unsafe integers and unlabelled or Mainnet values", () => {
  expect(schemas.financials.safeParse({ ...financials, simulated: false }).success).toBe(false);
  expect(schemas.financials.safeParse({ ...financials, balance_microusd: Number.MAX_SAFE_INTEGER + 1 }).success).toBe(false);
  expect(schemas.balances.safeParse({ network: "mainnet" }).success).toBe(false);
  expect(units(1234567890, 9)).toBe("1.23456789"); expect(units(-1000)).toBe("−0.001");
  expect(units(undefined)).toBe("Unavailable"); expect(units(Number.MAX_SAFE_INTEGER + 1)).toBe("Unavailable");
});
test("verified payment rows retain exact mint, network and evidence while stripping unknown data", () => {
  const proof = { network: "devnet", signature: "fixture-signature", mint: "4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU", amount_atoms: 1234567, slot: 42, evidence_source: "mock_rpc" };
  const row = { ...proof, id: agentId, data: proof };
  const [parsed] = schemas.payments.parse([row]);
  expect(parsed).toEqual(proof);
  expect(schemas.payments.safeParse([{ ...row, amount_atoms: 1234568 }]).success).toBe(false);
  expect(schemas.payments.safeParse([{ ...row, mint: "untrusted-mint" }]).success).toBe(false);
  expect(schemas.payments.safeParse([{ ...row, data: { ...proof, network: "mainnet" } }]).success).toBe(false);
});
test("API projections strip unrequested upstream fields", () => {
  expect(schemas.agent.parse({ agent: { ...agent, private_context: "not-public" }, financials, debug: "not-public" })).toEqual({ agent, financials });
});
