import { Button, Modal, Skeleton, toast } from "@roost/ui";
import { useEffect, useState } from "react";

import { type CollectionTls, collectionTlsRead, collectionTlsSave, toAppError } from "../../ipc";

/** The collection's own CA and client certificate (FR-5.3). */
export function TlsDialog({ root, onClose }: { root: string; onClose: () => void }) {
  const [tls, setTls] = useState<CollectionTls | null>(null);
  useEffect(() => {
    collectionTlsRead(root).then(setTls, (caught: unknown) => {
      toast.error(`Could not read the TLS settings: ${toAppError(caught).message}`);
    });
  }, [root]);

  const save = async () => {
    if (!tls) return;
    try {
      await collectionTlsSave(root, tls);
      toast.success("TLS settings saved");
      onClose();
    } catch (caught) {
      toast.error(`Could not save the TLS settings: ${toAppError(caught).message}`);
    }
  };
  const field = (key: keyof CollectionTls, label: string) =>
    tls && (
      <label className="flex flex-col gap-1 text-sm">
        {label}
        <input
          aria-label={label}
          className="input font-mono"
          placeholder="certs/file.pem"
          value={tls[key]}
          onChange={(e) => {
            setTls({ ...tls, [key]: e.target.value });
          }}
        />
      </label>
    );

  return (
    <Modal
      title="TLS"
      onClose={onClose}
      footer={
        <>
          <Button onClick={onClose}>Cancel</Button>
          <Button variant="primary" disabled={!tls} onClick={() => void save()}>
            Save
          </Button>
        </>
      }
    >
      {tls ? (
        <div className="flex flex-col gap-3">
          <p className="text-xs text-zinc-400">
            PEM files, relative to the collection (they may be git-ignored) or absolute. A PKCS#12
            file converts with
            <code className="mx-1">openssl pkcs12 -in client.p12 -out client.pem -nodes</code>.
          </p>
          {field("caFile", "Extra trusted CA")}
          {field("clientCertFile", "Client certificate and key")}
        </div>
      ) : (
        <Skeleton rows={2} label="Loading the TLS settings" />
      )}
    </Modal>
  );
}
