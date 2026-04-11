import { Streamdown } from "streamdown";

interface ReleaseNotesMarkdownProps {
  notes: string;
}

export function ReleaseNotesMarkdown({ notes }: ReleaseNotesMarkdownProps) {
  return (
    <div className="text-sm leading-6 text-text-primary [&_h2]:mt-4 [&_h2]:text-base [&_h2]:font-semibold [&_h2]:text-text-primary [&_h3]:mt-3 [&_h3]:text-sm [&_h3]:font-semibold [&_h3]:text-text-primary [&_p]:my-2 [&_ul]:my-2 [&_ul]:list-disc [&_ul]:pl-5 [&_ol]:my-2 [&_ol]:list-decimal [&_ol]:pl-5 [&_li]:my-1 [&_a]:text-glow-green [&_a]:underline [&_code]:rounded-md [&_code]:bg-space-overlay/70 [&_code]:px-1.5 [&_code]:py-0.5 [&_code]:text-[0.85em] [&_pre]:my-3 [&_pre]:overflow-x-auto [&_pre]:rounded-lg [&_pre]:bg-space-deep/70">
      <Streamdown>{notes}</Streamdown>
    </div>
  );
}
