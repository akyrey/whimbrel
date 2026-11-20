import { Button, Modal, Skeleton, toast } from "@roost/ui";
import { useEffect, useState } from "react";

import { type Header, nodeHeadersRead, nodeHeadersSave, toAppError } from "../../ipc";
import { HeaderRows } from "./HeaderRows";
import { useHeadersVersion } from "./headersVersion";

type Props = { root: string; folder: string; name: string; onClose: () => void };

/** The headers of a folder, or of the collection (`folder` empty), that every request under it
 * sends (D-H22). */
export function HeadersDialog({ root, folder, name, onClose }: Props) {
  const [headers, setHeaders] = useState<Header[] | null>(null);
  useEffect(() => {
    nodeHeadersRead(root, folder).then(setHeaders, (caught: unknown) => {
      toast.error(`Could not read the headers: ${toAppError(caught).message}`);
    });
  }, [root, folder]);

  const save = async () => {
    if (!headers) return;
    try {
      await nodeHeadersSave(root, folder, headers);
      useHeadersVersion.getState().bump();
      toast.success("Headers saved");
      onClose();
    } catch (caught) {
      toast.error(`Could not save the headers: ${toAppError(caught).message}`);
    }
  };

  return (
    <Modal
      title={`Headers of ${name}`}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!headers} onClick={() => void save()}>
            Save
          </Button>
        </>
      }
    >
      <p className="mb-2 text-xs text-zinc-400">
        Sent by every request under {name}. A request, or a folder inside, replaces one by adding a
        header of the same name; a disabled one turns it off.
      </p>
      {headers ? (
        <HeaderRows headers={headers} onChange={setHeaders} />
      ) : (
        <Skeleton rows={2} label="Loading the headers" />
      )}
    </Modal>
  );
}
