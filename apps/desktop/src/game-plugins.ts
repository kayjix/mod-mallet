// The plugins of the games themselves, apart from any mod.

const GAME_PLUGIN =
  /^(skyrim|update|dawnguard|hearthfires|dragonborn|fallout4|dlc\w*|oblivion|fallout3|falloutnv)\.esm$|^cc\w*\.es[lm]$/i;

/** Whether a plugin is one of the game's own. */
export function isGamePlugin(plugin: string): boolean {
  return GAME_PLUGIN.test(plugin);
}
