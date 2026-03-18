import { PanelShell } from "@/components/panels/PanelShell";
import { AboutPanel } from "@/features/about/AboutPanel";

export default function AboutView() {
  return (
    <PanelShell title="About Peekoo">
      <AboutPanel />
    </PanelShell>
  );
}
