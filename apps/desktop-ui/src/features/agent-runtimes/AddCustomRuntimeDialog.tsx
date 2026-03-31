import { useState } from "react";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";

interface AddCustomRuntimeDialogProps {
  isOpen: boolean;
  onClose: () => void;
  onSubmit: (payload: {
    name: string;
    description?: string;
    command: string;
    args: string[];
    workingDir?: string;
  }) => Promise<void>;
}

export function AddCustomRuntimeDialog({
  isOpen,
  onClose,
  onSubmit,
}: AddCustomRuntimeDialogProps) {
  const [name, setName] = useState("");
  const [description, setDescription] = useState("");
  const [command, setCommand] = useState("");
  const [args, setArgs] = useState("");
  const [workingDir, setWorkingDir] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [isSaving, setIsSaving] = useState(false);

  const reset = () => {
    setName("");
    setDescription("");
    setCommand("");
    setArgs("");
    setWorkingDir("");
    setError(null);
  };

  const handleClose = () => {
    reset();
    onClose();
  };

  const handleSubmit = async () => {
    if (!name.trim() || !command.trim()) {
      setError("Name and command are required.");
      return;
    }

    setIsSaving(true);
    setError(null);
    try {
      await onSubmit({
        name: name.trim(),
        description: description.trim() || undefined,
        command: command.trim(),
        args: args
          .split(" ")
          .map((part) => part.trim())
          .filter(Boolean),
        workingDir: workingDir.trim() || undefined,
      });
      handleClose();
    } catch (err) {
      setError(String(err));
    } finally {
      setIsSaving(false);
    }
  };

  return (
    <Dialog open={isOpen} onOpenChange={(open: boolean) => !open && handleClose()}>
      <DialogContent className="sm:max-w-lg">
        <DialogHeader>
          <DialogTitle>Add Custom ACP Runtime</DialogTitle>
          <DialogDescription>
            Register a local ACP command and then configure its LLM provider and models.
          </DialogDescription>
        </DialogHeader>

        <div className="space-y-4 py-4">
          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            <Label>Runtime Name</Label>
            <Input value={name} onChange={(e) => setName(e.target.value)} placeholder="My Custom ACP Runtime" />
          </label>

          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            <Label>Description</Label>
            <Input
              value={description}
              onChange={(e) => setDescription(e.target.value)}
              placeholder="Optional description"
            />
          </label>

          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            <Label>Command</Label>
            <Input value={command} onChange={(e) => setCommand(e.target.value)} placeholder="e.g. claude-code-acp" />
          </label>

          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            <Label>Arguments</Label>
            <Input
              value={args}
              onChange={(e) => setArgs(e.target.value)}
              placeholder="e.g. --acp --experimental-acp"
            />
          </label>

          <label className="flex flex-col gap-1 text-sm text-text-secondary">
            <Label>Working Directory</Label>
            <Input
              value={workingDir}
              onChange={(e) => setWorkingDir(e.target.value)}
              placeholder="Optional working directory"
            />
          </label>

          {error ? <p className="text-sm text-red-700 dark:text-red-300">{error}</p> : null}
        </div>

        <DialogFooter>
          <Button variant="outline" onClick={handleClose} disabled={isSaving}>
            Cancel
          </Button>
          <Button onClick={() => void handleSubmit()} disabled={isSaving}>
            {isSaving ? "Adding..." : "Add Runtime"}
          </Button>
        </DialogFooter>
      </DialogContent>
    </Dialog>
  );
}
