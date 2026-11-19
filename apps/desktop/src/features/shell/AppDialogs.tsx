import { CurlImportDialog } from "../curl/CurlImportDialog";
import { InteropImportDialog } from "../interop/InteropImportDialog";
import { SpecImportDialog } from "../openapi/SpecImportDialog";
import { SettingsDialog } from "../settings/SettingsDialog";
import { useDialogs } from "./dialogsStore";

/** The app-wide dialog that is open, if any. */
export function AppDialogs() {
  const open = useDialogs((s) => s.open);
  const close = useDialogs((s) => s.close);
  switch (open) {
    case "curl":
      return <CurlImportDialog onClose={close} />;
    case "openapi":
      return <SpecImportDialog onClose={close} />;
    case "interop":
      return <InteropImportDialog onClose={close} />;
    case "settings":
      return <SettingsDialog onClose={close} />;
    case null:
      return null;
  }
}
