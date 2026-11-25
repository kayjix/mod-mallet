// The catalogue of things (ADR 0054): every character, weapon, piece of
// armour and other thing of the game and the mods, alone or as the sets
// characters wear, on the same stage as the comparison of duels.

import { type ReactNode, useCallback, useEffect, useMemo, useState } from "react";
import { ActionResult, useAction, useSession } from "./Actions";
import {
  type CatalogueTexts,
  GROUP_OF,
  type Group,
  outfitWords,
  useCatalogueTexts,
} from "./catalogue-texts";
import type { AssetVersion, AssetVersions } from "./generated/asset-versions";
import type {
  Catalogue,
  CatalogueItem,
  CatalogueOutfit,
  CatalogueSource,
} from "./generated/catalogue";
import type { Action, ModRef } from "./generated/rendered-report";
import { InfoTip } from "./info-tip";
import { LoadingLine, LoadingScreen } from "./LoadingMallet";
import { MeshThumb } from "./mesh-thumb";
import { Stage, type StageItem } from "./Viewer3D";
import "./catalogue.css";
import { PageHeader, SearchField, Segmented, Tabs, Toolbar } from "./ui/kit";

type Arrange = "category" | "material" | "outfit";

/** Where things come from, as the filter has it; `mod:` names one mod. */
type Origin = "all" | "chosen" | "game" | "creation-club" | "mods" | "changed" | `mod:${string}`;

type Selection = { kind: "item"; id: string } | { kind: "outfit"; id: string };

const GROUPS: Group[] = ["characters", "weapons", "apparel", "items", "other"];

/** How many rows a section of the list shows before *show more*. */
const PAGE = 150;

/** How many weapons a set puts on the stage. */
const STAGE_WEAPONS = 2;

/** How many versions of a thing stand side by side; the others are picked. */
const SIDE_BY_SIDE = 4;

/** What the page knows about the catalogue, looked up once. */
interface Index {
  catalogue: Catalogue;
  items: Map<string, CatalogueItem>;
  outfits: Map<string, CatalogueOutfit>;
  sources: Map<string, CatalogueSource>;
  /** The sets each thing is part of. */
  setsOf: Map<string, CatalogueOutfit[]>;
  /** The mods that bring or change things, with how many. */
  mods: [string, number][];
  /** The name the mod manager shows for each mod, by its identity. */
  modNames: Map<string, string>;
}

function indexOf(catalogue: Catalogue, modNames: Map<string, string>): Index {
  const items = new Map(catalogue.items.map((item) => [item.id, item]));
  const outfits = new Map(catalogue.outfits.map((outfit) => [outfit.id, outfit]));
  const sources = new Map(catalogue.sources.map((source) => [source.plugin, source]));
  const setsOf = new Map<string, CatalogueOutfit[]>();
  for (const outfit of catalogue.outfits) {
    for (const id of [...outfit.worn, ...outfit.weapons]) {
      setsOf.set(id, [...(setsOf.get(id) ?? []), outfit]);
    }
  }
  const counts = new Map<string, number>();
  for (const item of catalogue.items) {
    const mods = new Set<string>();
    for (const plugin of [item.plugin, ...(item.changedBy ?? [])]) {
      const source = sources.get(plugin);
      if (source?.origin === "mod") {
        mods.add(modName(source, modNames));
      }
    }
    for (const mod of mods) {
      counts.set(mod, (counts.get(mod) ?? 0) + 1);
    }
  }
  const mods = [...counts].sort(([a], [b]) => a.localeCompare(b));
  return { catalogue, items, outfits, sources, setsOf, mods, modNames };
}

/** The name of the mod that brings a plugin, or the plugin's own name. */
function modName(source: CatalogueSource, modNames: Map<string, string>): string {
  return source.mod === null || source.mod === undefined
    ? source.plugin
    : modLabel(source.mod, modNames);
}

/** The name the mod manager shows for a mod, else its folder's. */
export function modLabel(mod: ModRef, modNames: Map<string, string>): string {
  return modNames.get(mod.id) ?? mod.name;
}

function nameOf(item: CatalogueItem): string {
  return item.name ?? item.editorId ?? item.id;
}

/** The name of a set: its body piece, or else its internal name in words. */
function outfitTitle(outfit: CatalogueOutfit, index: Index): string {
  const body = outfit.worn
    .map((id) => index.items.get(id))
    .find((item) => item?.category === "body");
  if (body !== undefined) {
    return nameOf(body);
  }
  return outfit.editorId === null || outfit.editorId === undefined
    ? outfit.id
    : outfitWords(outfit.editorId);
}

/** Whether a thing comes from where the filter asks. */
function fromOrigin(
  item: CatalogueItem,
  origin: Origin,
  index: Index,
  choices: Record<string, string>,
): boolean {
  const source = index.sources.get(item.plugin);
  const changers = (item.changedBy ?? []).map((plugin) => index.sources.get(plugin));
  const changedByMod = changers.some((changer) => changer?.origin === "mod");
  switch (origin) {
    case "all":
      return true;
    case "chosen":
      return choices[item.id] !== undefined;
    case "game":
      return source?.origin === "game" && !changedByMod;
    case "creation-club":
      return source?.origin === "creationClub";
    case "mods":
      return source?.origin === "mod";
    case "changed":
      return changedByMod;
    default: {
      const wanted = origin.slice("mod:".length);
      return [source, ...changers].some(
        (found) => found?.origin === "mod" && modName(found, index.modNames) === wanted,
      );
    }
  }
}

function matchesSearch(item: CatalogueItem, wanted: string): boolean {
  return (
    wanted === "" ||
    (item.name ?? "").toLowerCase().includes(wanted) ||
    (item.editorId ?? "").toLowerCase().includes(wanted)
  );
}

export function CatalogueView({ scannedAt }: { scannedAt: number }) {
  const { api, lang, mods } = useSession();
  const c = useCatalogueTexts();
  const [catalogue, setCatalogue] = useState<Catalogue | null | "loading">("loading");
  const [group, setGroup] = useState<Group>("weapons");
  const [arrange, setArrange] = useState<Arrange>("category");
  const [origin, setOrigin] = useState<Origin>("all");
  const [search, setSearch] = useState("");
  const [selected, setSelected] = useState<Selection | null>(null);
  // The versions of things the user put into the game, by thing.
  const [choices, setChoices] = useState<Record<string, string>>({});
  useEffect(() => {
    api.assetChoices().then(setChoices, () => setChoices({}));
  }, [api]);
  const choose = useCallback(
    (id: string, version: string | null) => {
      api.setAssetChoice(id, version).then(setChoices, () => undefined);
    },
    [api],
  );

  useEffect(() => {
    let live = true;
    // The scan is named so that a new scan reads the catalogue again.
    void scannedAt;
    api.catalogue(lang).then(
      (found) => live && setCatalogue(found),
      () => live && setCatalogue(null),
    );
    return () => {
      live = false;
    };
  }, [api, lang, scannedAt]);

  const index = useMemo(
    () =>
      catalogue === null || catalogue === "loading"
        ? null
        : indexOf(
            catalogue,
            new Map([...mods.values()].map((info) => [info.mod.id, info.mod.name])),
          ),
    [catalogue, mods],
  );

  if (catalogue === "loading") {
    return <LoadingScreen title={c.loading} />;
  }
  if (index === null) {
    return <p className="aside catalogue-message">{c.unreadable}</p>;
  }

  // Materials say nothing about characters or the things to carry.
  const arrangements: Arrange[] =
    group === "weapons" || group === "apparel"
      ? ["category", "material", "outfit"]
      : ["category", "outfit"];
  const shownArrange = arrangements.includes(arrange) ? arrange : "category";

  return (
    <>
      <PageHeader
        tabs={
          <Tabs
            label={c.groupsLabel}
            items={GROUPS.map((candidate) => ({ id: candidate, label: c.groups[candidate] }))}
            value={group}
            onChange={setGroup}
          />
        }
      />
      <div className="catalogue">
        <aside className="catalogue-left">
          <Toolbar stacked className="catalogue-filters">
            <SearchField value={search} onChange={setSearch} placeholder={c.search} wide />
            <label className="catalogue-field">
              <span>{c.originLabel}</span>
              <select value={origin} onChange={(event) => setOrigin(event.target.value as Origin)}>
                <option value="all">{c.originAll}</option>
                <option value="chosen">{c.originChosen}</option>
                <option value="game">{c.originGame}</option>
                <option value="creation-club">{c.originCreationClub}</option>
                <option value="mods">{c.originMods}</option>
                <option value="changed">{c.originChanged}</option>
                {index.mods.length > 0 && (
                  <optgroup label="Mods">
                    {index.mods.map(([mod, count]) => (
                      <option key={mod} value={`mod:${mod}`}>
                        {c.originMod(mod)} ({count})
                      </option>
                    ))}
                  </optgroup>
                )}
              </select>
            </label>
            <Segmented
              label={c.arrangeLabel}
              items={arrangements.map((candidate) => ({
                id: candidate,
                label:
                  candidate === "category"
                    ? group === "characters"
                      ? c.byRace
                      : c.byCategory
                    : candidate === "material"
                      ? c.byMaterial
                      : c.byOutfit,
              }))}
              value={shownArrange}
              onChange={setArrange}
            />
          </Toolbar>
          {shownArrange === "outfit" ? (
            <OutfitList
              index={index}
              choices={choices}
              group={group}
              origin={origin}
              search={search.trim().toLowerCase()}
              selected={selected}
              onSelect={setSelected}
            />
          ) : (
            <ItemList
              index={index}
              choices={choices}
              group={group}
              arrange={shownArrange}
              origin={origin}
              search={search.trim().toLowerCase()}
              selected={selected}
              onSelect={setSelected}
            />
          )}
        </aside>
        {selected === null ? (
          <p className="aside catalogue-message catalogue-pick">{c.pickSomething}</p>
        ) : selected.kind === "item" ? (
          <ItemView
            index={index}
            id={selected.id}
            onSelect={setSelected}
            choices={choices}
            onChoice={choose}
          />
        ) : (
          <OutfitView index={index} id={selected.id} onSelect={setSelected} />
        )}
      </div>
    </>
  );
}

// ---- left: the list ------------------------------------------------------------

interface ListProps {
  index: Index;
  choices: Record<string, string>;
  group: Group;
  origin: Origin;
  search: string;
  selected: Selection | null;
  onSelect: (selection: Selection) => void;
}

function ItemList({
  index,
  choices,
  group,
  arrange,
  origin,
  search,
  selected,
  onSelect,
}: ListProps & { arrange: "category" | "material" }) {
  const c = useCatalogueTexts();
  const sections = useMemo(() => {
    const found = new Map<string, CatalogueItem[]>();
    for (const item of index.catalogue.items) {
      if (
        GROUP_OF[item.category] !== group ||
        !matchesSearch(item, search) ||
        !fromOrigin(item, origin, index, choices)
      ) {
        continue;
      }
      const key = sectionOf(item, arrange, c);
      found.set(key, [...(found.get(key) ?? []), item]);
    }
    return [...found].sort(([a], [b]) => a.localeCompare(b));
  }, [index, choices, group, arrange, origin, search, c]);

  if (sections.length === 0) {
    return <p className="aside">{c.nothingFound}</p>;
  }
  const selectedId = selected?.kind === "item" ? selected.id : null;
  return (
    <ul className="catalogue-list">
      {sections.map(([title, items]) => (
        <Section
          key={title}
          title={title}
          count={items.length}
          // A search, or the thing looked at, keeps its section open.
          startOpen={search !== "" || sections.length === 1}
          holdsSelected={items.some((item) => item.id === selectedId)}
        >
          <Paged
            rows={items.map((item) => (
              <li key={item.id}>
                <ItemRow
                  item={item}
                  index={index}
                  chosen={choices[item.id] !== undefined}
                  selected={item.id === selectedId}
                  onClick={() => onSelect({ kind: "item", id: item.id })}
                />
              </li>
            ))}
          />
        </Section>
      ))}
    </ul>
  );
}

/** The heading a thing is listed under. */
function sectionOf(item: CatalogueItem, arrange: "category" | "material", c: CatalogueTexts) {
  if (item.category === "character") {
    return item.race ?? c.noRace;
  }
  if (arrange === "material") {
    return item.material === null || item.material === undefined
      ? c.noMaterial
      : c.material(item.material);
  }
  return c.categories[item.category];
}

function Section({
  title,
  count,
  startOpen,
  holdsSelected,
  children,
}: {
  title: string;
  count: number;
  startOpen: boolean;
  holdsSelected: boolean;
  children: ReactNode;
}) {
  const [open, setOpen] = useState(startOpen);
  const shown = open || startOpen || holdsSelected;
  return (
    <li className="catalogue-section">
      <button
        type="button"
        className="catalogue-section-head"
        aria-expanded={shown}
        onClick={() => setOpen(!shown)}
      >
        <span aria-hidden="true">{shown ? "▾" : "▸"}</span>
        <span>{title}</span>
        <span className="aside">{count}</span>
      </button>
      {shown && <ul className="catalogue-rows">{children}</ul>}
    </li>
  );
}

/** Rows shown a page at a time, so that long sections stay quick. */
function Paged({ rows }: { rows: ReactNode[] }) {
  const c = useCatalogueTexts();
  const [pages, setPages] = useState(1);
  const limit = pages * PAGE;
  return (
    <>
      {rows.slice(0, limit)}
      {rows.length > limit && (
        <li>
          <button type="button" className="link-button" onClick={() => setPages(pages + 1)}>
            {c.showMore(Math.min(PAGE, rows.length - limit))}
          </button>
        </li>
      )}
    </>
  );
}

function ItemRow({
  item,
  index,
  chosen,
  selected,
  onClick,
}: {
  item: CatalogueItem;
  index: Index;
  chosen: boolean;
  selected: boolean;
  onClick: () => void;
}) {
  const c = useCatalogueTexts();
  const source = index.sources.get(item.plugin);
  const changers = (item.changedBy ?? [])
    .map((plugin) => index.sources.get(plugin))
    .filter((found) => found?.origin === "mod");
  return (
    <button type="button" className="catalogue-row" aria-current={selected} onClick={onClick}>
      <span className="catalogue-row-name">{nameOf(item)}</span>
      <span className="catalogue-row-from">
        {source?.origin === "mod" && <span>{modName(source, index.modNames)}</span>}
        {changers.length > 0 && <span className="catalogue-changed">✎ {changers.length}</span>}
        {chosen && <span className="catalogue-chosen">✓ {c.versionChosen}</span>}
      </span>
    </button>
  );
}

function OutfitList({ index, choices, group, origin, search, selected, onSelect }: ListProps) {
  const c = useCatalogueTexts();
  const outfits = useMemo(() => {
    // A set is listed with the things the page lists: under weapons, sets
    // whose wearers carry weapons; otherwise every set.
    return index.catalogue.outfits
      .filter((outfit) => group !== "weapons" || outfit.weapons.length > 0)
      .filter((outfit) => {
        const pieces = [...outfit.worn, ...outfit.weapons]
          .map((id) => index.items.get(id))
          .filter((item): item is CatalogueItem => item !== undefined);
        const searched =
          search === "" ||
          (outfit.editorId ?? "").toLowerCase().includes(search) ||
          outfitTitle(outfit, index).toLowerCase().includes(search) ||
          pieces.some((piece) => matchesSearch(piece, search));
        return (
          searched &&
          (origin === "all" || pieces.some((piece) => fromOrigin(piece, origin, index, choices)))
        );
      })
      .sort(
        (a, b) => b.characters - a.characters || (a.editorId ?? "").localeCompare(b.editorId ?? ""),
      );
  }, [index, choices, group, origin, search]);

  if (outfits.length === 0) {
    return <p className="aside">{c.nothingFound}</p>;
  }
  const selectedId = selected?.kind === "outfit" ? selected.id : null;
  return (
    <ul className="catalogue-list catalogue-rows">
      <Paged
        rows={outfits.map((outfit) => (
          <li key={outfit.id}>
            <button
              type="button"
              className="catalogue-row catalogue-set-row"
              aria-current={outfit.id === selectedId}
              onClick={() => onSelect({ kind: "outfit", id: outfit.id })}
            >
              <span className="catalogue-row-name">{outfitTitle(outfit, index)}</span>
              <span className="catalogue-row-from">
                <span>{c.setPieces(outfit.worn.length, outfit.weapons.length)}</span>
                {outfit.characters > 0 && <span>{c.worn(outfit.characters)}</span>}
              </span>
              {outfit.editorId && <span className="catalogue-internal">{outfit.editorId}</span>}
            </button>
          </li>
        ))}
      />
    </ul>
  );
}

// ---- middle and right: one thing, or one set ---------------------------------------

function ItemView({
  index,
  id,
  onSelect,
  choices,
  onChoice,
}: {
  index: Index;
  id: string;
  onSelect: (selection: Selection) => void;
  choices: Record<string, string>;
  onChoice: (id: string, version: string | null) => void;
}) {
  const c = useCatalogueTexts();
  const item = index.items.get(id);
  if (item === undefined) {
    return <p className="aside catalogue-message catalogue-pick">{c.pickSomething}</p>;
  }
  return (
    <ItemPage
      key={item.id}
      item={item}
      index={index}
      onSelect={onSelect}
      choices={choices}
      onChoice={onChoice}
    />
  );
}

/** The name of a version: the game, the mod, the archive or loose files. */
function versionLabel(version: AssetVersion, index: Index, c: CatalogueTexts): string {
  switch (version.source.type) {
    case "game":
      return c.game;
    case "mod":
      return modLabel(version.source.mod, index.modNames);
    case "archive":
      return version.source.name;
    default:
      return c.looseFiles;
  }
}

function ItemPage({
  item,
  index,
  onSelect,
  choices,
  onChoice,
}: {
  item: CatalogueItem;
  index: Index;
  onSelect: (selection: Selection) => void;
  choices: Record<string, string>;
  onChoice: (id: string, version: string | null) => void;
}) {
  const c = useCatalogueTexts();
  const { api } = useSession();
  const action = useAction();
  // The version being put into the game, or `null` while the choice is
  // being taken back; remembered once the mod manager has done it.
  const [pending, setPending] = useState<string | null | undefined>(undefined);
  useEffect(() => {
    if (pending === undefined || action.busy || action.outcome === null) {
      return;
    }
    if (action.outcome.outcome.status === "applied") {
      onChoice(item.id, pending);
    }
    setPending(undefined);
  }, [action.busy, action.outcome, pending, item.id, onChoice]);
  const choose = (version: string | null, choice: Action) => {
    setPending(version);
    void action.run(choice);
  };
  const [versions, setVersions] = useState<AssetVersions | null | undefined>(undefined);
  const [model, setModel] = useState(0);
  // The versions on the stage; until the user picks, the first ones.
  const [chosen, setChosen] = useState<string[] | null>(null);
  useEffect(() => {
    let live = true;
    api.assetVersions(item.models).then(
      (found) => live && setVersions(found),
      () => live && setVersions(null),
    );
    return () => {
      live = false;
    };
  }, [api, item]);

  const all = versions?.versions ?? [];
  const onStage = (chosen ?? all.slice(0, SIDE_BY_SIDE).map((version) => version.id)).filter((id) =>
    all.some((version) => version.id === id),
  );
  const path = item.models[model] ?? item.models[0] ?? "";
  const onStageKey = onStage.join("|");
  // biome-ignore lint/correctness/useExhaustiveDependencies: the key stands for the chosen versions
  const stage = useMemo<StageItem[]>(() => {
    if (versions === undefined) {
      return [];
    }
    const shown = all.filter((version) => onStage.includes(version.id));
    if (shown.length === 0) {
      return [{ kind: "mesh", label: nameOf(item), path }];
    }
    return shown.map((version) => ({
      kind: "mesh",
      label: versionLabel(version, index, c),
      path,
      assetVersion: version.id,
    }));
  }, [versions, onStageKey, path, item, index, c]);
  const toggle = (id: string) => {
    setChosen(onStage.includes(id) ? onStage.filter((known) => known !== id) : [...onStage, id]);
  };

  const outfit =
    item.outfit === null || item.outfit === undefined ? undefined : index.outfits.get(item.outfit);
  const sets = index.setsOf.get(item.id) ?? [];
  const models =
    item.category === "character" ? [c.face] : item.models.length > 1 ? [c.onGround, c.asWorn] : [];
  return (
    <>
      <section className="catalogue-middle">
        <div className="catalogue-middle-head">
          <h2 className="catalogue-title">{nameOf(item)}</h2>
          {models.length > 1 && (
            <fieldset className="segmented" aria-label={c.modelLabel}>
              {models.map((label, at) => (
                <button
                  key={label}
                  type="button"
                  aria-pressed={model === at}
                  onClick={() => setModel(at)}
                >
                  {label}
                </button>
              ))}
            </fieldset>
          )}
        </div>
        {versions === undefined ? (
          <LoadingLine className="catalogue-message" text={c.versionsLoading} />
        ) : (
          <Stage key={`${item.id}/${path}`} items={stage} />
        )}
        {all.length > 0 && (
          <div className="catalogue-versions">
            <div className="catalogue-versions-head">
              <h3>{c.versionsTitle}</h3>
              {choices[item.id] !== undefined && versions?.reset && (
                <button
                  type="button"
                  disabled={action.busy}
                  title={c.resetHint}
                  onClick={() => versions.reset && choose(null, versions.reset)}
                >
                  {c.reset}
                </button>
              )}
            </div>
            <p className="aside">
              {all.length === 1 ? c.versionsOnlyOne : c.versionsHint(SIDE_BY_SIDE)}
            </p>
            <ul>
              {all.map((version) => {
                const on = onStage.includes(version.id);
                const meshes = version.files.filter((file) => file.startsWith("meshes/")).length;
                const textures = version.files.length - meshes;
                return (
                  <li key={version.id}>
                    <label
                      className="catalogue-version"
                      data-shown={
                        version.shown.length === 0
                          ? "no"
                          : version.shown.length === version.files.length
                            ? "all"
                            : "some"
                      }
                    >
                      <input
                        type="checkbox"
                        checked={on}
                        disabled={all.length === 1 || (!on && onStage.length >= SIDE_BY_SIDE)}
                        onChange={() => toggle(version.id)}
                      />
                      <MeshThumb
                        className="catalogue-version-thumb"
                        path={path}
                        version={version.id}
                      />
                      <span className="catalogue-version-name">
                        {versionLabel(version, index, c)}
                      </span>
                      <span className="catalogue-version-files">
                        {c.versionBrings(meshes, textures)}
                      </span>
                      {version.shown.length > 0 && (
                        <span className="catalogue-version-shown">
                          {version.shown.length === version.files.length
                            ? c.versionInGame
                            : c.versionPartlyInGame(version.shown.length, version.files.length)}
                        </span>
                      )}
                      {choices[item.id] === version.id && (
                        <span className="catalogue-version-chosen">{c.versionChosen}</span>
                      )}
                    </label>
                    {version.choice && (
                      <button
                        type="button"
                        className="catalogue-version-take"
                        disabled={action.busy}
                        title={c.takeHint}
                        onClick={() => version.choice && choose(version.id, version.choice)}
                      >
                        {c.take}
                      </button>
                    )}
                  </li>
                );
              })}
            </ul>
            <ActionResult state={action} />
          </div>
        )}
      </section>
      <aside className="catalogue-right">
        <dl className="catalogue-details">
          <dt>{item.category === "character" ? c.detailRace : c.detailCategory}</dt>
          <dd>
            {item.category === "character" ? (item.race ?? c.noRace) : c.categories[item.category]}
          </dd>
          {item.material && (
            <>
              <dt>{c.detailMaterial}</dt>
              <dd>{c.material(item.material)}</dd>
            </>
          )}
          <dt>{c.detailFrom}</dt>
          <dd>
            <SourceName plugin={item.plugin} index={index} />
          </dd>
          {(item.changedBy ?? []).length > 0 && (
            <>
              <dt>{c.detailChangedBy}</dt>
              <dd>
                <ol className="catalogue-changers">
                  {(item.changedBy ?? []).map((plugin) => (
                    <li key={plugin}>
                      <SourceName plugin={plugin} index={index} />
                    </li>
                  ))}
                </ol>
              </dd>
            </>
          )}
          {item.editorId && (
            <>
              <dt>{c.detailInternal}</dt>
              <dd className="catalogue-internal">{item.editorId}</dd>
            </>
          )}
        </dl>
        {(item.variants ?? 0) > 0 && (
          <p className="aside">{c.detailVariants(item.variants ?? 0)}</p>
        )}
        {outfit !== undefined && (
          <>
            <h3>{c.detailOutfit}</h3>
            <SetLinks outfits={[outfit]} index={index} onSelect={onSelect} />
          </>
        )}
        {sets.length > 0 && (
          <>
            <h3>{c.detailInOutfits}</h3>
            <SetLinks outfits={sets} index={index} onSelect={onSelect} />
          </>
        )}
        <h3>{c.detailModels}</h3>
        <ul className="catalogue-files">
          {item.models.map((path) => (
            <li key={path}>{path}</li>
          ))}
        </ul>
      </aside>
    </>
  );
}

function OutfitView({
  index,
  id,
  onSelect,
}: {
  index: Index;
  id: string;
  onSelect: (selection: Selection) => void;
}) {
  const c = useCatalogueTexts();
  const outfit = index.outfits.get(id);
  if (outfit === undefined) {
    return <p className="aside catalogue-message catalogue-pick">{c.pickSomething}</p>;
  }
  const pieces = (ids: string[]) =>
    ids
      .map((piece) => index.items.get(piece))
      .filter((item): item is CatalogueItem => item !== undefined);
  const worn = pieces(outfit.worn);
  const weapons = pieces(outfit.weapons);
  const stage: StageItem[] = [...worn, ...weapons.slice(0, STAGE_WEAPONS)].flatMap((item) =>
    item.models.slice(0, 1).map((path) => ({ kind: "mesh" as const, label: nameOf(item), path })),
  );
  const pieceList = (items: CatalogueItem[]) => (
    <ul className="catalogue-pieces">
      {items.map((item) => (
        <li key={item.id}>
          <button
            type="button"
            className="link-button"
            onClick={() => onSelect({ kind: "item", id: item.id })}
          >
            {nameOf(item)}
          </button>
          <span className="aside"> · {c.categories[item.category]}</span>
        </li>
      ))}
    </ul>
  );
  return (
    <>
      <section className="catalogue-middle">
        <h2 className="catalogue-title">{outfitTitle(outfit, index)}</h2>
        <Stage key={outfit.id} items={stage} />
      </section>
      <aside className="catalogue-right">
        <dl className="catalogue-details">
          <dt>{c.detailFrom}</dt>
          <dd>
            <SourceName plugin={outfit.plugin} index={index} />
          </dd>
          {outfit.editorId && (
            <>
              <dt>{c.detailInternal}</dt>
              <dd className="catalogue-internal">{outfit.editorId}</dd>
            </>
          )}
        </dl>
        {outfit.characters > 0 && <p className="aside">{c.worn(outfit.characters)}</p>}
        <h3>{c.detailWorn}</h3>
        {pieceList(worn)}
        {weapons.length > 0 && (
          <>
            <h3>
              {c.detailWeapons} <InfoTip text={c.detailWeaponsHint} />
            </h3>
            {pieceList(weapons)}
          </>
        )}
      </aside>
    </>
  );
}

function SetLinks({
  outfits,
  index,
  onSelect,
}: {
  outfits: CatalogueOutfit[];
  index: Index;
  onSelect: (selection: Selection) => void;
}) {
  return (
    <ul className="catalogue-pieces">
      {outfits.map((outfit) => (
        <li key={outfit.id}>
          <button
            type="button"
            className="link-button"
            onClick={() => onSelect({ kind: "outfit", id: outfit.id })}
          >
            {outfitTitle(outfit, index)}
          </button>
          {outfit.editorId && <span className="catalogue-internal"> {outfit.editorId}</span>}
        </li>
      ))}
    </ul>
  );
}

/** Where a plugin comes from, in words: the game, the Creation Club or the mod. */
function SourceName({ plugin, index }: { plugin: string; index: Index }) {
  const c = useCatalogueTexts();
  const source = index.sources.get(plugin);
  const name =
    source?.origin === "game"
      ? c.game
      : source?.origin === "creationClub"
        ? c.creationClub
        : source === undefined
          ? plugin
          : modName(source, index.modNames);
  return (
    <span>
      {name} <span className="catalogue-internal">{plugin}</span>
    </span>
  );
}
