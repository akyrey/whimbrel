import { Button, Modal } from "@roost/ui";
import { useState } from "react";

type Props = {
  title: string;
  label: string;
  initial: string;
  confirmLabel: string;
  onSubmit: (name: string) => void;
  onCancel: () => void;
};

/** Asks for a name (new request, new folder, rename, new collection). */
export function NameDialog({ title, label, initial, confirmLabel, onSubmit, onCancel }: Props) {
  const [name, setName] = useState(initial);
  const trimmed = name.trim();
  return (
    <Modal
      title={title}
      onClose={onCancel}
      footer={
        <>
          <Button onClick={onCancel}>Cancel</Button>
          <Button
            variant="primary"
            disabled={trimmed === ""}
            onClick={() => {
              onSubmit(trimmed);
            }}
          >
            {confirmLabel}
          </Button>
        </>
      }
    >
      <form
        onSubmit={(e) => {
          e.preventDefault();
          if (trimmed !== "") onSubmit(trimmed);
        }}
      >
        <input
          aria-label={label}
          autoFocus
          className="input w-full"
          value={name}
          onChange={(e) => {
            setName(e.target.value);
          }}
        />
      </form>
    </Modal>
  );
}
