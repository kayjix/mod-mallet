### Texte, die nicht zu einer einzelnen Befundart gehören: Beschriftungen und
### Berichtszeilen für Kommandozeile und Desktop-App.

severity-error = Fehler
severity-warning = Warnung
severity-info = Hinweis

confidence-certain = sicher
confidence-likely = wahrscheinlich
confidence-hint = Vermutung

label-file = Datei
label-mod = Mod
label-remedy = Was tun
label-kind = Art
label-severity = Schwere
label-confidence = Sicherheit

location-game-dir = Spielordner
location-plugin-list = Liste der aktiven Plugins
location-extender-log = Protokoll des Script Extenders
location-deployment-manifest = Vortex' Liste der bereitgestellten Dateien
location-manager-state = Vortex' gespeicherter Zustand
location-downloads = Vortex' heruntergeladene Mod-Archive
location-unpacker = Programm zum Hineinsehen in Mod-Archive
location-masterlist = LOOTs Liste bekannter Plugin-Probleme (Masterliste)
location-crash-logs = Ordner mit Absturzberichten
location-mod-staging = Vortex' Ordner der installierten Mods (die Namen ihrer Dateien, und die Dateien der Dinge, die du unter Assets ansiehst)
origin-given = von dir angegeben
origin-registry = aus dem Eintrag, den der Launcher des Spiels in die Windows-Registry schreibt
origin-standard = am üblichen Ort
origin-steam = in einer deiner Steam-Bibliotheken gefunden
origin-gog = aus dem Eintrag, den GOG Galaxy in die Windows-Registry schreibt
origin-manager = genannt in Vortex' Liste der bereitgestellten Dateien
location-not-found = nicht gefunden

report-game = { $name } { $version }
report-game-unknown-version = { $name } (Version unbekannt)
report-extender = Script Extender { $version }
report-plugins-checked =
    { $count ->
        [one] 1 Script-Extender-Plugin geprüft
       *[other] { $count } Script-Extender-Plugins geprüft
    }
report-files-read =
    { $plugins ->
        [one] 1 Plugin
       *[other] { $plugins } Plugins
    } und { $archives ->
        [one] 1 Archiv
       *[other] { $archives } Archive
    } gelesen
report-installers-read =
    { $count ->
        [one] 1 Mod-Installer gelesen
       *[other] { $count } Mod-Installer gelesen
    }
report-meshes-compared =
    { $count ->
        [one] 1 Ersatz-Modell mit dem Original verglichen
       *[other] { $count } Ersatz-Modelle mit dem Original verglichen
    }
report-no-extender-log = Der Script Extender hat kein Protokoll geschrieben, seine Plugins wurden deshalb nicht geprüft.
report-no-findings = Keine Probleme gefunden.
report-totals =
    { $errors ->
        [one] 1 Fehler
       *[other] { $errors } Fehler
    }, { $warnings ->
        [one] 1 Warnung
       *[other] { $warnings } Warnungen
    }, { $notes ->
        [one] 1 Hinweis
       *[other] { $notes } Hinweise
    }
report-notes-hidden = Hinweise sind nicht aufgelistet. Mit --notes werden sie angezeigt.
report-explain-hint = Mehr zu einem Befund: mallet explain <Art>
report-no-scan = Es ist noch kein Scan gespeichert. Starte zuerst "mallet scan".

explain-unknown-kind = Eine Befundart namens "{ $kind }" gibt es nicht. "mallet kinds" listet alle auf.

## Aktionen: was sich gegen einen Befund tun lässt und was daraus wurde.

label-action = Von hier beheben

action-reinstall-mod = In Vortex neu installieren
    .description = Vortex öffnet den Installer von { $mod } erneut, mit deiner früheren Auswahl vorbelegt. Geh ihn Seite für Seite durch und kreuze die hier aufgeführten Optionen an; das passiert nicht von selbst. Es ändert sich nichts, bis du den Installer dort abschließt.

guide-page = Seite { $page } von { $pages }

action-hide-objects = Patch in Vortex installieren
    .description = Mod Mallet schreibt eine kleine Mod, die im Spiel { $count ->
        [one] 1 Objekt
       *[other] { $count } Objekte
    } durch einen unsichtbaren Marker ersetzt, und Vortex installiert sie. Dafür braucht es Base Object Swapper. Entfernst du die Mod in Vortex, sind die Objekte wieder da.

action-load-after = { $plugin } gewinnen lassen
    .description = Vortex bekommt eine Regel, { $plugin } nach { $after } zu laden, und sortiert deine Plugins neu. Was beide ändern, kommt dann aus { $plugin }; was nur { $after } ändert, bleibt. Die Regel findest du in Vortex' Plugin-Liste und kannst sie dort wieder entfernen.

action-disable-mod = { $mod } deaktivieren
    .description = Vortex schaltet { $mod } im verwendeten Profil ab und bittet ums Bereitstellen. Gelöscht wird nichts; schalte die Mod in Vortex wieder ein, um es rückgängig zu machen.

action-enable-mod = { $mod } einschalten
    .description = Vortex schaltet { $mod } im verwendeten Profil ein und bittet ums Bereitstellen. Schalte die Mod in Vortex wieder ab, um es rückgängig zu machen.

action-file-overrides = Fassung von { $mod } nehmen
    .description = Vortex lässt { $count } { $count ->
        [one] Datei
       *[other] Dateien
    } dieses Gegenstands bei den anderen Mods weg, die sie mitbringen, sodass die von { $mod } ins Spiel kommen, und stellt neu bereit. Nur diese Dateien wechseln. In Vortex findest du es unter den Dateikonflikten der Mods und kannst es dort zurücknehmen.

action-file-overrides-archives = Fassung aus den Archiven nehmen
    .description = Vortex lässt { $count } { $count ->
        [one] Datei
       *[other] Dateien
    } dieses Gegenstands bei den Mods weg, die sie lose mitbringen, sodass die Fassung aus den Archiven zu sehen ist, und stellt neu bereit. In Vortex findest du es unter den Dateikonflikten der Mods und kannst es dort zurücknehmen.

action-file-overrides-reset = Wahl zurücknehmen
    .description = Vortex stellt { $count } { $count ->
        [one] Datei
       *[other] Dateien
    } dieses Gegenstands wieder von allen Mods bereit, die sie mitbringen; dann entscheidet wieder die Reihenfolge der Mods.

## Duelle: alles, worin zwei Mods sich uneins sind.

duel-topic-faces = { $count ->
    [one] 1 Gesicht
   *[other] { $count } Gesichter
}
duel-topic-textures = { $count ->
    [one] 1 Textur
   *[other] { $count } Texturen
}
duel-topic-meshes = { $count ->
    [one] 1 Mesh
   *[other] { $count } Meshes
}
duel-topic-objects = { $count ->
    [one] 1 Objekt
   *[other] { $count } Objekte
}
duel-topic-contained = { $count ->
    [one] 1 enthaltenes Plugin
   *[other] { $count } enthaltene Plugins
}
duel-topic-files = { $count ->
    [one] 1 andere Datei
   *[other] { $count } andere Dateien
}
duel-subject-faces = Gesichter
duel-subject-textures = Texturen
duel-subject-meshes = Meshes
duel-subject-objects = platzierte Objekte
duel-subject-contained = Plugins
duel-subject-files = andere Dateien
duel-suggest-built-on = { $winner } baut auf { $other } auf: Es wurde gemacht, um es zu ändern, und sollte gewinnen.
duel-suggest-specialist = Bei { $winner } dreht sich fast alles um { $subject }, bei { $other } sind sie nur ein kleiner Teil. Der Spezialist kann sein Thema meist besser.

outcome-applied = Vortex hat die Änderung vorgenommen. Stelle bereit, falls Vortex darum bittet; Mod Mallet scannt danach erneut.
outcome-installing = Vortex installiert den Patch. Stelle bereit, falls Vortex das nicht von selbst tut; Mod Mallet scannt danach erneut.
outcome-started = Vortex hat den Installer geöffnet. Mach dort weiter und scanne danach erneut.
outcome-link-missing = Dafür braucht es eine kleine Erweiterung in Vortex, die noch nicht installiert ist.
outcome-manager-not-running = Vortex antwortet nicht. Starte Vortex, oder starte es neu, falls die Erweiterung gerade erst installiert wurde, und versuche es noch einmal.
outcome-no-answer = Vortex hat nicht rechtzeitig geantwortet. Prüfe, ob Vortex geöffnet ist und nicht in einem Dialog wartet, und versuche es noch einmal.
outcome-wrong-game = Vortex verwaltet gerade ein anderes Spiel. Stelle Vortex auf dieses Spiel um und versuche es noch einmal.
outcome-unknown-mod = Vortex kennt diese Mod nicht mehr. Scanne erneut.
outcome-archive-missing = Vortex hat das Archiv nicht mehr, aus dem diese Mod installiert wurde. Lade die Mod zuerst in Vortex erneut herunter.
outcome-unsupported = Die Erweiterung in Vortex kennt diese Anfrage nicht. Installiere die Erweiterung neu und starte Vortex neu.
outcome-failed = Das hat nicht geklappt: { $detail }

link-ready = Die Vortex-Erweiterung ist installiert und Vortex antwortet.
link-not-running = Die Vortex-Erweiterung ist installiert, aber Vortex antwortet nicht. Starte Vortex, oder starte es neu, falls die Erweiterung gerade erst installiert wurde.
link-restart-needed = In Vortex läuft noch eine ältere Fassung der Erweiterung. Starte Vortex neu, damit die neue geladen wird.
link-missing = Die Vortex-Erweiterung ist nicht installiert.
link-explanation = Mod Mallet verändert deine Mods nicht selbst. Damit Vortex das übernehmen kann, legt Mod Mallet eine kleine Erweiterung in Vortex' Plugin-Ordner. Die Erweiterung handelt nur, wenn du in Mod Mallet einen Knopf drückst, und tut dann dasselbe wie Vortex' eigener Menüeintrag.
link-installed = Die Erweiterung wurde nach { $path } installiert. Starte Vortex neu, damit sie geladen wird.

## Der Werkzeugkasten: was das Setup neben den Mods braucht.

tool-vortex-name = Vortex
tool-vortex-purpose = Der Mod-Manager. Mod Mallet liest, welche Mod welche Datei mitgebracht hat, und überlässt jede Änderung Vortex.
tool-mallet-link-name = Mod-Mallet-Verbindung in Vortex
tool-mallet-link-purpose = Eine kleine Vortex-Erweiterung. Über sie können die Knöpfe in Mod Mallet Vortex bitten, eine Mod neu zu installieren oder einen Patch zu installieren.
tool-skse-name = Skyrim Script Extender (SKSE)
tool-skse-purpose = Erlaubt Mods, dem Spiel Programmcode hinzuzufügen. Braucht eine Fassung genau für deine Spielversion; Steam-Updates legen ihn lahm, bis SKSE nachzieht.
tool-address-library-name = Address Library for SKSE Plugins
tool-address-library-purpose = Sagt SKSE-Plugins, wo in deiner Spielversion was liegt, damit sie kleine Spielupdates überstehen. Die meisten SKSE-Plugins brauchen sie.
tool-vc-runtime-name = Microsoft Visual C++ Redistributable
tool-vc-runtime-purpose = Ein Windows-Baustein, mit dem fast jedes Script-Extender-Plugin gebaut ist. Ohne ihn laden diese Plugins nicht.
tool-base-object-swapper-name = Base Object Swapper
tool-base-object-swapper-purpose = Ein SKSE-Plugin, das Objekte austauscht, während das Spiel läuft. Die Patches von Mod Mallet blenden damit kollidierende Objekte aus.
tool-creation-kit-name = Creation Kit
tool-creation-kit-purpose = Bethesdas Editor. Du brauchst ihn, um die Wege zu reparieren, auf denen Figuren laufen (Navigationsnetze) – Mod Mallet kann nur darauf zeigen –, um einen Ort so anzusehen, wie das Spiel ihn baut, mit seinem Licht, und um eigene Mods zu bauen. Zum Spielen und für Mod Mallet nicht nötig.
tool-skse-vr-name = Skyrim VR Script Extender (SKSEVR)
tool-skse-vr-purpose = Erlaubt Mods, Skyrim VR Programmcode hinzuzufügen. Braucht eine Fassung genau für deine Spielversion.
tool-skse-classic-name = Skyrim Script Extender (SKSE)
tool-skse-classic-purpose = Erlaubt Mods, dem Skyrim von 2011 Programmcode hinzuzufügen. Das Spiel ändert sich nicht mehr, also passt eine Fassung.
tool-f4se-name = Fallout 4 Script Extender (F4SE)
tool-f4se-purpose = Erlaubt Mods, dem Spiel Programmcode hinzuzufügen. Braucht eine Fassung genau für deine Spielversion; Steam-Updates legen ihn lahm, bis F4SE nachzieht.
tool-f4se-vr-name = Fallout 4 VR Script Extender (F4SEVR)
tool-f4se-vr-purpose = Erlaubt Mods, Fallout 4 VR Programmcode hinzuzufügen. Braucht eine Fassung genau für deine Spielversion.
tool-nvse-name = New Vegas Script Extender (xNVSE)
tool-nvse-purpose = Erlaubt Mods, dem Spiel Programmcode hinzuzufügen. Die meisten größeren Mods für New Vegas brauchen ihn.
tool-fose-name = Fallout Script Extender (FOSE)
tool-fose-purpose = Erlaubt Mods, Fallout 3 Programmcode hinzuzufügen. Viele größere Mods brauchen ihn.
tool-obse-name = Oblivion Script Extender (OBSE)
tool-obse-purpose = Erlaubt Mods, Oblivion Programmcode hinzuzufügen. Viele größere Mods brauchen ihn.
tool-obse64-name = Oblivion Remastered Script Extender (OBSE64)
tool-obse64-purpose = Erlaubt Mods, dem alten Spiel in Oblivion Remastered Programmcode hinzuzufügen. Läuft nur mit der Steam-Version und braucht eine Version, die genau zu deiner Spielversion passt.
tool-ue4ss-name = UE4SS
tool-ue4ss-purpose = Lädt Mods für die Unreal-Engine-Seite von Oblivion Remastered: die im Ordner LogicMods und Mods mit Skripten. Mods, die nur Bilder oder Modelle ersetzen, brauchen es nicht.
tool-address-library-vr-name = VR Address Library for SKSEVR
tool-address-library-vr-purpose = Sagt Plugins, die für Skyrim Special Edition gebaut sind, wo in Skyrim VR was liegt, damit viele von ihnen auch dort laufen.
tool-address-library-f4se-name = Address Library for F4SE Plugins
tool-address-library-f4se-purpose = Sagt F4SE-Plugins, wo in deiner Spielversion was liegt, damit sie kleine Spielupdates überstehen. Die meisten F4SE-Plugins brauchen sie.
tool-geck-name = G.E.C.K.
tool-geck-purpose = Bethesdas Editor für Fallout 3 und New Vegas. Du brauchst ihn, um die Wege zu reparieren, auf denen Figuren laufen (Navigationsnetze), um einen Ort mit seinem Licht anzusehen und um eigene Mods zu bauen. Zum Spielen und für Mod Mallet nicht nötig.
tool-construction-set-name = Construction Set
tool-construction-set-purpose = Bethesdas Editor für Oblivion. Du brauchst ihn, um einen Ort mit seinem Licht anzusehen und um eigene Mods zu bauen. Zum Spielen und für Mod Mallet nicht nötig.
tool-loot-name = LOOT
tool-loot-purpose = Sortiert Plugins und führt eine Liste bekannter Probleme mit ihnen. Vortex hat LOOT eingebaut und hält diese Liste aktuell; Mod Mallet liest sie, um deine Plugins zu prüfen. Ohne Vortex leistet LOOT als eigenes Programm dasselbe.
tool-seven-zip-name = 7-Zip
tool-seven-zip-purpose = Öffnet Mod-Archive. Mod Mallet liest damit die Installer deiner Mods; Vortex bringt eines mit.
tool-xedit-name = xEdit (SSEEdit)
tool-xedit-purpose = Schaut in Plugins hinein und kann sie reparieren. Die geführten Varianten säubern ein Plugin von Überresten, die das Spiel abstürzen lassen, oder bauen einen Patch, mit dem zwei Mods zusammenarbeiten. Starte es über Vortex' Übersicht, damit es deine Mods sieht.
tool-xedit-clean-name = xEdit Quick Auto Clean
tool-xedit-clean-purpose = Säubert ein Plugin mit wenigen Klicks von Überresten, die das Spiel abstürzen lassen können. Mod Mallet weist darauf hin, wenn LOOT sagt, dass ein Plugin gesäubert werden muss. Starte es über Vortex' Übersicht, damit es deine Mods sieht.
tool-wrye-bash-name = Wrye Bash
tool-wrye-bash-purpose = Fasst die kleinen Änderungen vieler Mods in einem Plugin zusammen (ein Bashed Patch), damit sie zusammenspielen und du unter der Plugin-Grenze bleibst.
tool-fnis-name = Fores New Idles in Skyrim (FNIS)
tool-fnis-purpose = Macht dem Spiel die neuen Animationen bekannt, die Mods mitbringen, etwa neue Geh- oder Kampfstile. Es muss nach dem Hinzufügen oder Entfernen solcher Mods erneut laufen, sonst stehen die Figuren steif da.
tool-bodyslide-name = BodySlide
tool-bodyslide-purpose = Baut die Körper und Kleidung von Mods, die in mehreren Formen kommen, nach deinem Geschmack. Das Ergebnis entsteht bei jeder Änderung neu.
tool-mod-mallet-tool-name = Mod Mallet
tool-mod-mallet-tool-purpose = Lässt dich Mod Mallet über Vortex' Übersicht starten, neben deinen anderen Werkzeugen. Vortex ändert seine Liste nur, wenn du hier den Knopf drückst.
tool-vortex-tool-purpose = Ein Programm, das Vortex unter deinen Werkzeugen aufführt. Mod Mallet weiß nicht, wofür es gut ist; starte es über Vortex' Übersicht.

tool-state-ready = Bereit
tool-state-missing = Fehlt
tool-state-outdated = Braucht ein Update
tool-state-stopped = Läuft nicht
tool-need-required = Nötig
tool-need-recommended = Empfohlen
tool-need-optional = Optional
tool-provided-by = In { $tool } eingebaut
