import { Button, Modal, Skeleton, toast } from "@roost/ui";
import { useEffect, useState } from "react";

import { type Auth, nodeAuthRead, nodeAuthSave, toAppError } from "../../ipc";
import { AuthEditor } from "./AuthEditor";
import { useAuthVersion } from "./authVersion";

type Props = { root: string; folder: string; name: string; onClose: () => void };

/** The auth of a folder, or of the collection (`folder` empty), that requests inherit. */
export function AuthDialog({ root, folder, name, onClose }: Props) {
  const [auth, setAuth] = useState<Auth | null>(null);
  useEffect(() => {
    nodeAuthRead(root, folder).then(setAuth, (caught: unknown) => {
      toast.error(`Could not read the auth: ${toAppError(caught).message}`);
    });
  }, [root, folder]);

  const save = async () => {
    if (!auth) return;
    try {
      await nodeAuthSave(root, folder, auth);
      useAuthVersion.getState().bump();
      toast.success("Auth saved");
      onClose();
    } catch (caught) {
      toast.error(`Could not save the auth: ${toAppError(caught).message}`);
    }
  };

  return (
    <Modal
      title={`Auth of ${name}`}
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!auth} onClick={() => void save()}>
            Save
          </Button>
        </>
      }
    >
      {auth ? (
        <AuthEditor auth={auth} onChange={setAuth} allowInherit={folder !== ""} />
      ) : (
        <Skeleton rows={2} label="Loading the auth" />
      )}
    </Modal>
  );
}
