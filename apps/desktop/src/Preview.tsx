import { useEffect, useState } from "react";
import { useSession } from "./Actions";
import { isPictured } from "./compare-files";
import type { FilePreview } from "./generated/file-preview";
import { LoadingLine } from "./LoadingMallet";
import { type MeshItem, Viewer3D } from "./Viewer3D";

/**
 * What the files of a finding look like: for each, every version the setup
 * has side by side, the one the game uses marked. Loaded when shown.
 */
export function Previews({ paths }: { paths: string[] }) {
  const { t, api, mods } = useSession();
  // Vortex names a deployed file by the mod's folder; the report knows the
  // mod's proper name by the same identity.
  const nameOf = (id: string, fallback: string) =>
    [...mods.values()].find((info) => info.mod.id === id)?.mod.name ?? fallback;
  const [previews, setPreviews] = useState<FilePreview[] | null>(null);
  const [viewing, setViewing] = useState<MeshItem[] | null>(null);

  useEffect(() => {
    let outdated = false;
    // Scripts, menus and other files are compared on the assets page; only
    // textures and meshes have pictures.
    const pictured = paths.filter(isPictured);
    Promise.all(pictured.map((path) => api.filePreview(path).catch(() => null)))
      .then((found) => {
        if (!outdated) {
          setPreviews(found.filter((preview): preview is FilePreview => preview !== null));
        }
      })
      .catch(() => {});
    return () => {
      outdated = true;
    };
  }, [api, paths]);

  if (previews === null) {
    return <LoadingLine text={t.previewLoading} />;
  }
  const shown = previews.filter((preview) => preview.versions.length > 0);
  if (shown.length === 0) {
    return null;
  }
  return (
    <div className="previews">
      <h3>{t.previewTitle}</h3>
      {shown.map((preview) => (
        <div className="preview" key={preview.path}>
          <code className="preview-path">{preview.path.replace(/^data\//, "")}</code>
          {preview.path.toLowerCase().endsWith(".nif") ? (
            <div className="preview-meshes">
              {preview.versions.map((version, index) => {
                const label =
                  version.provider == null
                    ? (version.archive ?? t.previewLoose)
                    : nameOf(version.provider.id, version.provider.name);
                return (
                  <button
                    type="button"
                    key={version.archive ?? "loose"}
                    onClick={() => setViewing([{ label, path: preview.path, version: index }])}
                  >
                    {t.view3d}: {label}
                    {index === preview.versions.length - 1 && ` · ${t.previewInGame}`}
                  </button>
                );
              })}
              {preview.versions.length > 1 && (
                <button
                  type="button"
                  className="primary"
                  onClick={() =>
                    setViewing(
                      preview.versions.slice(-2).map((version, offset) => ({
                        label:
                          version.provider == null
                            ? (version.archive ?? t.previewLoose)
                            : nameOf(version.provider.id, version.provider.name),
                        path: preview.path,
                        version: preview.versions.length - 2 + offset,
                      })),
                    )
                  }
                >
                  {t.view3dBoth}
                </button>
              )}
            </div>
          ) : (
            <div className="preview-versions">
              {preview.versions.map((version, index) => {
                const inGame = index === preview.versions.length - 1;
                const label =
                  version.provider == null
                    ? (version.archive ?? t.previewLoose)
                    : nameOf(version.provider.id, version.provider.name);
                return (
                  <figure
                    className="preview-version"
                    data-in-game={inGame}
                    key={version.archive ?? "loose"}
                  >
                    {version.image == null ? (
                      <div className="preview-missing">{t.previewNoPicture}</div>
                    ) : (
                      <img src={version.image} alt={label} />
                    )}
                    <figcaption>
                      <span>{label}</span>
                      {inGame && <span className="preview-badge">{t.previewInGame}</span>}
                    </figcaption>
                  </figure>
                );
              })}
            </div>
          )}
        </div>
      ))}
      {viewing !== null && <Viewer3D items={viewing} onClose={() => setViewing(null)} />}
    </div>
  );
}
