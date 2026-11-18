### Texte für jede Befundart aus `mallet-core/src/kinds.rs`.
###
### Die Regeln stehen in `locales/en/findings.ftl`. Englisch ist die
### Ausgangssprache: Dort kommt eine neue Befundart zuerst hinein.

extender-plugin-incompatible = Script-Extender-Plugin passt nicht zur Spielversion
    .headline = { $file } passt nicht zur Spielversion { $gameVersion }
    .summary = { $file } wurde abgeschaltet, weil es nicht für die Spielversion { $gameVersion } gebaut ist.
    .explanation = Der Script Extender lädt native Plugins (DLL-Dateien), mit denen Mods Funktionen nachrüsten, die das Spiel allein nicht bietet. Jedes Plugin gibt an, welche Spielversionen es unterstützt. Dieses unterstützt deine Version nicht, deshalb hat der Script Extender es nicht geladen. Das Spiel startet trotzdem, aber alles, was das Plugin bereitstellt, fehlt. Andere Mods, die darauf aufbauen, können sich falsch verhalten.
    .remedy = Sieh auf der Seite der Mod nach, ob es eine Version für deine Spielversion gibt, und installiere sie. Gibt es noch keine, deaktiviere die Mod, bis sie aktualisiert wurde.

extender-plugin-not-loaded = Script-Extender-Plugin wurde nicht geladen
    .headline = { $file } wurde nicht geladen
    .summary = { $file } wurde nicht geladen. Der Script Extender meldet: { $reason }
    .explanation = Der Script Extender wollte dieses native Plugin (eine DLL-Datei) laden und hat abgebrochen. Das Spiel startet trotzdem, aber alles, was das Plugin bereitstellt, fehlt. Der Grund ist wörtlich aus dem Protokoll des Script Extenders übernommen. Häufige Ursachen sind ein Plugin für eine andere Spielversion, eine fehlende Voraussetzung wie die Address Library oder ein beschädigter Download.
    .remedy = Prüfe, ob die Voraussetzungen der Mod installiert sind und ob die Mod zu deiner Spielversion passt, und installiere sie dann neu. Hilft das nicht, deaktiviere die Mod und melde dem Autor den zitierten Grund.

extender-plugin-no-version-data = Datei im Plugin-Ordner ist kein Script-Extender-Plugin
    .headline = { $file } ist kein Script-Extender-Plugin
    .summary = { $file } liegt im Plugin-Ordner des Script Extenders, gibt sich aber nicht als Plugin zu erkennen und wurde übersprungen.
    .explanation = Manche Mods liefern neben ihrem Plugin Hilfsbibliotheken mit. Der Script Extender prüft jede DLL in seinem Plugin-Ordner und überspringt alle ohne Plugin-Angaben. Das ist harmlos. Ein Problem ist es nur, wenn die Datei eigentlich ein Plugin sein sollte: Dann ist sie beschädigt oder für ein anderes Spiel gebaut.
    .remedy = Nichts zu tun, wenn die Datei eine Hilfsbibliothek einer deiner Mods ist. Sollte sie ein Plugin sein, installiere die zugehörige Mod neu.

plugins-master-missing = Ein Plugin baut auf einem Plugin auf, das nicht installiert ist
    .headline =
        { $plugin }: { $count ->
            [one] 1 benötigtes Plugin fehlt
           *[other] { $count } benötigte Plugins fehlen
        }
    .summary =
        { $plugin } baut auf { $count ->
            [one] einem Plugin auf, das nicht installiert ist
           *[other] { $count } Plugins auf, die nicht installiert sind
        }: { $masters }.
    .explanation = Ein Plugin kann auf anderen Plugins aufbauen, seinen Mastern, und nennt sie in seiner Datei. Das Spiel lädt die Master zuerst. Fehlt einer davon, schließt sich das Spiel beim Start ohne Meldung, meist direkt nach dem Logo. Das passiert typischerweise, wenn ein Patch für zwei Mods installiert ist, eine der beiden aber nicht, oder wenn eine Mod entfernt wurde und ihre Patches geblieben sind.
    .remedy = Installiere die Mod, die das fehlende Plugin enthält, oder deaktiviere die Mod, zu der dieses Plugin gehört. Ist das Plugin ein Patch, den ein Installer angeboten hat, gehe diesen Installer noch einmal durch und wähle den Patch ab.

plugins-generated-outdated = Ein erzeugtes Plugin ist älter als die Ladereihenfolge
    .headline = { $plugin } wurde für eine ältere Ladereihenfolge erzeugt
    .summary =
        { $tool } hat { $plugin } erzeugt, bevor sich die Ladereihenfolge geändert hat. { $count ->
            [one] Ein Plugin, aus dem es erzeugt wurde, lädt jetzt
           *[other] { $count } Plugins, aus denen es erzeugt wurde, laden jetzt
        } an einer anderen Stelle: { $examples }.
    .explanation = Manche Werkzeuge schauen sich alle deine Plugins an und schreiben aus dem, was sie finden, ein eigenes Plugin: DynDOLOD für die Fernsicht, Synthesis und Wrye Bash für Patches, die die Änderungen vieler Mods zusammenführen. So ein Plugin ist eine Momentaufnahme. Ändert sich danach die Ladereihenfolge, weil eine Mod dazukam, entfernt oder neu sortiert wurde, enthält die Momentaufnahme noch den alten Stand. Objekte in der Ferne passen dann womöglich nicht zu dem, was du aus der Nähe siehst, und Änderungen neu sortierter Mods können verloren gehen.
    .remedy = Lass das Werkzeug, das das Plugin erzeugt hat, noch einmal laufen und ersetze seine alte Ausgabe durch die neue. Hältst du die Ausgabe als Mod in deinem Mod-Manager, installiere die neue Ausgabe in dieselbe Mod.

plugins-requirement-missing = Ein Plugin braucht eine Datei, die im Spiel fehlt
    .headline =
        { $plugin }: { $count ->
            [one] 1 benötigte Datei fehlt
           *[other] { $count } benötigte Dateien fehlen
        }
    .summary =
        Laut LOOT braucht { $plugin } { $count ->
            [one] eine Datei, die in deinem Spiel fehlt oder nicht eingeschaltet ist
           *[other] { $count } Dateien, die in deinem Spiel fehlen oder nicht eingeschaltet sind
        }: { $files }.
    .explanation = LOOT, das Werkzeug, das Plugins sortiert, führt eine Liste darüber, was Tausende Plugins brauchen und womit sie nicht zusammen funktionieren. Mod-Manager wie Vortex laden diese Liste von selbst herunter. Viele Mods funktionieren nur zusammen mit einer anderen Mod, oft einem Script-Extender-Plugin oder einer Bibliothek wie PapyrusUtil. Fehlt sie, tun Teile der Mod nichts, ihre Skripte schlagen fehl, oder das Spiel schließt sich, sobald es das fehlende Stück braucht.
    .remedy = Installiere die Mod, die die fehlende Datei mitbringt; auf der Seite der Mod steht, was sie braucht. Ist die fehlende Datei ein installiertes Plugin, schalte es in deinem Mod-Manager ein. Willst du die andere Mod nicht, deaktiviere die Mod, zu der dieses Plugin gehört.

plugins-incompatible = Zwei Mods funktionieren nicht zusammen
    .headline = { $plugin } funktioniert nicht zusammen mit { $other }
    .summary = Laut LOOT kann { $plugin } nicht zusammen mit { $other } benutzt werden, und beide sind in deinem Spiel.
    .explanation = Manche Mods ändern dasselbe auf eine Art, die sich gegenseitig ausschließt: zwei Fassungen derselben Fehlerkorrektur, zwei Überarbeitungen desselben Teils des Spiels oder eine alte Mod, deren Arbeit eine neuere schon enthält. LOOTs Liste nennt solche Paare. Sind beide installiert, macht die eine die andere rückgängig, Funktionen gehen kaputt oder das Spiel wird instabil.
    .remedy = Entscheide, welche der beiden du willst, und deaktiviere die andere in deinem Mod-Manager. Ersetzt die eine die andere, behalte die neuere. Auf den Seiten der Mods steht oft, welche du wählen solltest.

plugins-needs-cleaning = Ein Plugin löscht Dinge, auf die das Spiel und andere Mods angewiesen sind
    .headline = { $plugin } sollte bereinigt werden
    .summary = LOOT kennt genau diese Fassung von { $plugin }. Mit { $utility } gezählt: gelöschte Objekte: { $deleted }, gelöschte Navigationsnetze: { $navmeshes }, unveränderte Kopien von Einträgen: { $identical }.
    .explanation = Manche Plugins löschen Objekte des Spiels, statt sie auszuschalten, ein Überbleibsel davon, wie sie gebaut wurden. Verweist eine andere Mod oder ein Skript des Spiels auf ein gelöschtes Objekt, kann das Spiel abstürzen. Am schwersten wiegen gelöschte Navigationsnetze, die Wege, auf denen Figuren laufen: Sie können das Spiel abstürzen lassen und lassen sich nicht automatisch reparieren. LOOTs Liste hält fest, welche veröffentlichten Fassungen eines Plugins solche Löschungen enthalten. Sie erkennt eine Fassung an der Prüfsumme der Datei, es geht also genau um deine Datei.
    .remedy = Installiere eine neuere Fassung der Mod, wenn es eine gibt. Sonst lässt sich das Plugin mit „Quick Auto Clean“ von SSEEdit bereinigen, das gelöschte Objekte stattdessen ausschaltet; die Anleitungen von LOOT und SSEEdit erklären die Schritte. Lege vorher eine Kopie des Plugins an. Mit Vortex bereinigst du die Datei im Ordner der Mod in Vortex' Staging-Ordner, damit die bereinigte Datei bleibt. Gelöschte Navigationsnetze lassen sich nicht automatisch entfernen: Gibt es welche, such auf der Seite der Mod nach einer korrigierten Fassung oder einem Patch.

plugins-identical-records = Ein Plugin enthält unveränderte Kopien von Einträgen
    .headline =
        { $plugin } enthält { $count ->
            [one] 1 unveränderte Kopie
           *[other] { $count } unveränderte Kopien
        } von Einträgen des Spiels
    .summary =
        LOOT kennt genau diese Fassung von { $plugin }: Mit { $utility } gezählt enthält sie { $count ->
            [one] 1 Eintrag
           *[other] { $count } Einträge
        }, unverändert kopiert aus einem Plugin, auf dem sie aufbaut.
    .explanation = Ein Eintrag, den ein Plugin unverändert kopiert, bewirkt für sich nichts. Er zählt erst, wenn eine andere Mod, die früher lädt, denselben Eintrag ändert: Die unveränderte Kopie stellt dann still das Original wieder her. Das ist meist harmlos, kann aber die Änderung einer anderen Mod verdecken. LOOTs Liste hält fest, welche veröffentlichten Fassungen eines Plugins solche Kopien enthalten; sie erkennt eine Fassung an der Prüfsumme der Datei.
    .remedy = Meist ist nichts zu tun. Zeigt sich die Änderung einer anderen Mod nicht im Spiel, bereinige dieses Plugin mit „Quick Auto Clean“ von SSEEdit oder lass es vor dieser Mod laden.

plugins-loot-warning = LOOT warnt vor einem Plugin
    .headline = LOOT warnt vor { $plugin }
    .summary = LOOT sagt zu { $plugin }, in eigenen Worten (auf Englisch): { $messages }
    .explanation = LOOT, das Werkzeug, das Plugins sortiert, führt Hinweise zu Tausenden Plugins und zeigt sie, wenn sie auf ein Setup zutreffen: ein fehlender Kompatibilitätspatch, eine Mod, die nicht zusammen mit einer anderen benutzt werden sollte, ein Plugin, das gelöscht werden sollte, weil eine andere Mod es schon enthält. Mod-Manager wie Vortex zeigen diese Hinweise ebenfalls in ihrer Plugin-Liste. Mod Mallet zitiert sie so, wie LOOT sie schreibt, auf Englisch, und nur die, deren Bedingungen für dein Setup gelten.
    .remedy = Lies die Meldung und folge ihr. Ein fehlender Patch liegt meist auf der Seite, die die Meldung nennt, oder wird vom Installer einer der beiden Mods angeboten. Sagt die Meldung, ein Plugin solle gelöscht oder abgeschaltet werden, deaktiviere die Mod, zu der es gehört, in deinem Mod-Manager.

plugins-loot-note = LOOT hat einen Hinweis zu einem Plugin
    .headline =
        LOOT hat { $count ->
            [one] einen Hinweis
           *[other] { $count } Hinweise
        } zu { $plugin }
    .summary = LOOT sagt zu { $plugin }, in eigenen Worten (auf Englisch): { $messages }
    .explanation = LOOT, das Werkzeug, das Plugins sortiert, führt Hinweise zu Tausenden Plugins und zeigt sie, wenn sie auf ein Setup zutreffen: ein Update-Patch, der die Mod korrigiert, eine Patch-Sammlung für sie, lesenswerte Kompatibilitätshinweise oder dass die Mod einen neuen Spielstand braucht. Mod Mallet zitiert sie so, wie LOOT sie schreibt, auf Englisch, und nur die, deren Bedingungen für dein Setup gelten.
    .remedy = Es ist nichts zu tun. Sieh dir den Hinweis an, wenn du an dieser Mod etwas änderst oder sie sich nicht wie erwartet verhält.


npcs-face-mismatch = Charaktere mit dunklen oder unpassenden Gesichtern
    .headline =
        { $winnerPlugin } verändert { $count ->
            [one] 1 Gesicht
           *[other] { $count } Gesichter
        }, die für { $facePlugin } gemacht sind
    .summary =
        { $winnerPlugin } wird nach { $facePlugin } geladen und beschreibt für { $count ->
            [one] 1 Charakter ein anderes Gesicht
           *[other] { $count } Charaktere andere Gesichter
        } als die verwendeten Gesichtsdateien. Diese stammen aus { $faceOrigin ->
            [game] dem Spiel selbst
           *[mod] { $faceMod }
        }. Zum Beispiel: { $examples }.
    .explanation = Das Spiel baut das Gesicht eines Charakters nicht beim Spielen zusammen. Es lädt ein Gesichtsmodell, das vorab aus dem Eintrag des Charakters erzeugt wurde. Beschreibt der Eintrag, der in der Ladereihenfolge gewinnt, ein anderes Gesicht als das, für das dieses Modell erzeugt wurde, zeichnet das Spiel den Kopf in falschen Farben, meist deutlich dunkler als den Körper. Das passiert, wenn eine Mod die Gesichtsdateien liefert und ein später geladenes Plugin einer anderen Mod dieselben Charaktere ändert. Die Prüfung vergleicht die Einträge, wie sie geschrieben sind. Sie kann deshalb auch ein Plugin melden, dessen Änderung im Spiel nicht sichtbar ist.
    .remedy = Willst du das Aussehen der Gesichtsdateien, lade deren Plugin nach dem anderen oder suche einen Kompatibilitäts-Patch für die beiden Mods. Willst du die Charaktere so, wie das andere Plugin sie beschreibt, muss dessen Mod passende Gesichtsdateien mitliefern.

installer-options-now-recommended = Empfohlene Installer-Optionen sind nicht installiert
    .headline =
        { $mod }: { $count ->
            [one] 1 empfohlene Option fehlt
           *[other] { $count } empfohlene Optionen fehlen
        }
    .summary =
        Der Installer von { $mod } würde jetzt { $count ->
            [one] 1 Option ankreuzen, die bei der Installation nicht gewählt wurde
           *[other] { $count } Optionen ankreuzen, die bei der Installation nicht gewählt wurden
        }: { $examples }. Angegeben ist jeweils Schritt › Gruppe › Option.
    .explanation = Viele Mods bringen einen Installer mit, der Patches für andere Mods anbietet und diejenigen ankreuzt, die zu dem passen, was in diesem Moment installiert ist. Der Dialog erscheint nur einmal. Wird eine Mod, die so einen Patch braucht, erst danach installiert, bietet ihn niemand mehr an, und die beiden Mods laufen ungepatcht nebeneinander: Objekte stehen ineinander, Änderungen einer Mod gehen verloren, oder Licht und Texturen passen nicht zusammen. Mod Mallet spielt die Regeln des Installers gegen das aktuelle Setup durch und meldet die Optionen, die er jetzt ankreuzen würde. Eine Option, die du bewusst weggelassen hast, wird ebenfalls gemeldet.
    .remedy = Installiere die Mod in deinem Mod-Manager neu, gehe ihren Installer noch einmal durch und kreuze die oben genannten Optionen an. In Vortex: Mod auswählen und "Reinstall" wählen. Danach bereitstellen und die Plugins sortieren.

plugins-contained = Eine Mod steckt schon in einer anderen
    .headline = { $plugin } steckt schon in { $container }
    .summary =
        { $container } stellt { $count } der { $total } Objekte von { $plugin } an dieselben Stellen. { $plugin } ist darin also schon enthalten, und das Spiel zeigt diese Objekte doppelt. { $dependentCount ->
            [0] Kein anderes Plugin baut auf { $pluginFile } auf.
           *[other] Auf { $pluginFile } bauen diese Plugins auf: { $dependents }.
        }
    .explanation = Sammelpakete ("all in one") enthalten oft Mods, die es auch einzeln gibt. Sind beide installiert, stehen ihre Objekte doppelt an derselben Stelle: Oberflächen flimmern, Türen und Behälter können doppelt vorhanden sein. Oft braucht ein Patch die Einzel-Mod als Master. Dann lässt sie sich nicht einfach abschalten, sonst schließt sich das Spiel beim Start.
    .remedy = Baut kein anderes Plugin auf die enthaltene Mod auf, deaktiviere sie in deinem Mod-Manager. Baut ein Patch darauf auf, sieh auf der Seite des Patches nach einer Fassung für das Sammelpaket; viele Patches gibt es einmal für die Einzel-Mod und einmal für das Sammelpaket. Gibt es keine, lass vorerst beides aktiv und nimm die doppelten Objekte in Kauf.

objects-duplicated = Zwei Mods stellen dasselbe Objekt doppelt auf
    .headline =
        { $first } und { $second } stellen { $count ->
            [one] 1 Objekt
           *[other] { $count } Objekte
        } doppelt auf
    .summary =
        { $first } und { $second } stellen { $count ->
            [one] dasselbe Objekt
           *[other] dieselben { $count } Objekte
        } fast an dieselbe Stelle, das Spiel zeigt { $count ->
            [one] es
           *[other] sie
        } also doppelt. Zum Beispiel: { $examples }. Angegeben ist jeweils der Ort: ein Innenraum mit seinem internen Namen, draußen Weltraum und Zelle.
    .explanation = Mods, die dieselbe Stadt oder dieselbe Gegend verändern, fügen oft dieselben Dinge hinzu, etwa einen Zaun, einen Marktstand oder ein Haus, das dem Spiel selbst fehlt. Jede Mod stellt ihre eigene Kopie auf. Das Spiel zeigt beide Kopien an derselben Stelle: Wo sich die Oberflächen treffen, flimmert es, und Türen oder Behälter können doppelt erscheinen. Mod-Manager zeigen das nicht, weil die beiden Mods verschiedene Einträge ändern.
    .remedy = Sieh auf den Seiten beider Mods nach einem Kompatibilitäts-Patch; für bekannte Kombinationen gibt es meist einen. Installiere ihn zu beiden Mods dazu und scanne erneut. Gibt es keinen, entscheide, welche der beiden Mods du für diesen Ort behalten willst, oder nimm die doppelten Objekte in Kauf.

objects-overlapping = Objekte zweier Mods stehen ineinander
    .headline =
        { $first } und { $second }: { $count ->
            [one] 1 Objekt steht
           *[other] { $count } Objekte stehen
        } in einem anderen
    .summary =
        { $first } und { $second } stellen jeweils Objekte ähnlicher Größe an dieselbe Stelle, sodass sie an { $count ->
            [one] 1 Ort
           *[other] { $count } Orten
        } größtenteils ineinander stehen. Zum Beispiel: { $examples }. Angegeben ist jeweils der Ort: ein Innenraum mit seinem internen Namen, draußen Weltraum und Zelle.
    .explanation = Zwei Mods, die denselben Ort verändern, stellen jeweils eigene Gebäude, Bäume oder andere Objekte dorthin, ohne voneinander zu wissen. Wo sie sich treffen, wächst ein Baum durch ein Dach, zwei Häuser verschmelzen zu einem oder ein Weg führt in eine Wand. Wo sich die Modelle beider Objekte lesen lassen, vergleicht Mod Mallet ihre Formen; sonst nur den Kasten um jedes Modell. Ein Objekt mit viel freiem Raum, etwa ein Torbogen, kann dann gemeldet werden, obwohl sichtbar nichts zusammenstößt.
    .remedy = Sieh auf den Seiten beider Mods nach einem Kompatibilitäts-Patch. Gibt es keinen, sieh dir die genannten Orte im Spiel an. Steht dort etwas im Weg, entscheide, welche der beiden Mods du für diesen Ort behalten willst.

objects-touching = Objekte zweier Mods ragen ineinander
    .headline =
        { $first } und { $second }: { $count ->
            [one] 1 Objekt ragt
           *[other] { $count } Objekte ragen
        } in ein anderes
    .summary =
        Objekte von { $first } und von { $second } berühren sich oder ragen ein Stück ineinander, an { $count ->
            [one] 1 Ort
           *[other] { $count } Orten
        }: Ihre Modelle schneiden sich, aber keins steht größtenteils im anderen. Zum Beispiel: { $examples }. Angegeben ist jeweils der Ort: ein Innenraum mit seinem internen Namen, draußen Weltraum und Zelle.
    .explanation = Zwei Mods, die Dinge in dieselbe Gegend stellen, wussten nichts voneinander. So läuft ein Zaun in eine Mauer, ein Karren steht halb in einem Marktstand oder eine Laterne wächst aus einem Dach. Mod Mallet hat die Formen beider Modelle verglichen, nicht nur die Kästen um sie herum. Innerhalb einer Mod sind solche Berührungen meist gewollt, deshalb werden sie nur zwischen Mods gemeldet, die nichts voneinander wissen.
    .remedy = Sieh dir die genannten Orte im Spiel oder auf der Karte an. Oft fällt es nicht auf; wo doch, entscheide, welches Objekt du behalten willst, oder suche nach einem Kompatibilitäts-Patch beider Mods.

navmeshes-door-cut-off = Figuren können Türen einer Mod nicht mehr benutzen
    .headline =
        { $winner } schneidet { $count ->
            [one] 1 Tür
           *[other] { $count } Türen
        } von { $loser } ab
    .summary =
        { $loser } bindet { $count ->
            [one] eine Tür, die Mods hinzufügen,
           *[other] { $count } Türen, die Mods hinzufügen,
        } an die Wege an, auf denen Figuren laufen. { $winner } ersetzt diese Wege durch eine Fassung, die ohne { $loser } erstellt wurde und in der { $count ->
            [one] die Tür nicht angebunden ist
           *[other] die Türen nicht angebunden sind
        }. Zum Beispiel: { $places }. Angegeben ist ein Innenraum mit seinem internen Namen, draußen Weltraum und Zelle.
    .explanation = Das Spiel legt auf den Boden jedes Ortes ein unsichtbares Netz, über das Figuren ihren Weg finden: sein Navigationsnetz. Eine Tür können Figuren nur benutzen, wenn das Netz zu ihr führt. Eine Mod, die ein Haus hinzufügt, bindet das Netz an die Tür des Hauses an. Ersetzt eine spätere Mod dasselbe Netz durch ihre eigene Fassung, ist die Tür nicht mehr angebunden: Begleiter, Wachen und Bewohner bleiben davor stehen oder gehen nie hinein, und Quests, die jemanden hindurchschicken, können hängen bleiben. Nur du kannst sie noch öffnen.
    .remedy = Sieh auf den Seiten beider Mods nach einem Kompatibilitäts-Patch und installiere ihn, wenn es einen gibt. Ohne Patch entscheide, welche der beiden Mods du für diese Orte behalten willst. Die Mod mit der Tür später zu laden, verschiebt das Problem nur: Dann gehen die Wege der anderen Mod verloren.

navmeshes-changes-undone = Eine Mod macht rückgängig, wo eine andere Mod Figuren laufen lässt
    .headline =
        { $winner } macht { $count ->
            [one] 1 Laufweg
           *[other] { $count } Laufwege
        } von { $loser } rückgängig
    .summary =
        { $loser } und { $winner } ändern beide, wo Figuren { $count ->
            [one] an einem Ort
           *[other] an { $count } Orten
        } laufen können. Das Spiel verwendet nur die Fassung von { $winner }, die nicht mit { $loser } erstellt wurde. Zum Beispiel: { $places }. Angegeben ist ein Innenraum mit seinem internen Namen, draußen Weltraum und Zelle.
    .explanation = Das Spiel legt auf den Boden jedes Ortes ein unsichtbares Netz, über das Figuren ihren Weg finden: sein Navigationsnetz. Eine Mod, die ein Haus oder eine Mauer baut, ändert dieses Netz, damit die Leute darum herumgehen. Ein Navigationsnetz wird immer als Ganzes ersetzt. Ändern zwei Mods dasselbe, zählt nur die Fassung der späteren Mod. Menschen und Tiere laufen dann durch die Gebäude der früheren Mod, bleiben an ihren Mauern hängen oder finden den Weg in das, was sie hinzugefügt hat, nicht mehr. Begleiter zeigen es oft als Erste.
    .remedy = Sieh auf den Seiten beider Mods nach einem Kompatibilitäts-Patch und installiere ihn, wenn es einen gibt. Ohne Patch behalte für diese Orte nur eine der beiden Mods, oder sieh sie dir im Spiel an, um zu beurteilen, ob es dort stört.

meshes-made-for-other-game = Modelle einer Mod wurden für ein anderes Spiel gemacht
    .headline =
        { $count ->
            [one] 1 Modell
           *[other] { $count } Modelle
        } für { $madeFor }
    .summary =
        { $count ->
            [one] Ein Modell dieser Mod liegt
           *[other] { $count } Modelle dieser Mod liegen
        } im Format von { $madeFor } vor, nicht in dem des Spiels, das du spielst{ $skinned ->
            [0] .
            [one] . Eines davon bewegt sich mit einem Skelett, wie Kleidung oder Rüstung – stürzt etwas ab oder bleibt unsichtbar, sieh zuerst nach diesem.
           *[other] . { $skinned } davon bewegen sich mit einem Skelett, wie Kleidung oder Rüstung – stürzt etwas ab oder bleibt unsichtbar, sieh zuerst nach diesen.
        } Zum Beispiel: { $examples }.
    .explanation = Ein Modell ist die 3D-Form eines Dings im Spiel. Jedes Spiel und jede Ausgabe eines Spiels liest Modelle in einer eigenen Variante des Formats. Mods für das ursprüngliche Skyrim von 2011 sind oft mit einigen unveränderten Modellen in die Special Edition gekommen, und auch viele gepflegte Mods bringen solche Modelle mit. Die meisten funktionieren. Manche nicht: Dann bleibt das Ding, zu dem sie gehören, unsichtbar, sieht falsch aus, oder das Spiel schließt sich, sobald es erscheint – am ehesten bei Kleidung, Rüstungen und Kreaturen, die sich mit einem Skelett bewegen. Dieser Hinweis ist eine Spur für den Fall, dass so etwas passiert, kein Zeichen, dass es passieren wird.
    .remedy = Nichts zu tun, solange das Spiel gut läuft. Stürzt etwas dieser Mod das Spiel ab oder bleibt unsichtbar, sieh auf der Seite der Mod nach einer Fassung für dein Spiel und installiere stattdessen diese. Gibt es keine, lassen sich die Modelle mit Cathedral Assets Optimizer umwandeln, einem kostenlosen Werkzeug genau dafür. Installiere die umgewandelten Dateien dann als eigene Mod, die diese überschreibt.

meshes-texture-missing = Modelle einer Mod verwenden Texturen, die im Spiel fehlen
    .headline =
        { $count ->
            [one] 1 Textur fehlt
           *[other] { $count } Texturen fehlen
        } für { $meshes ->
            [one] 1 Modell
           *[other] { $meshes } Modelle
        }
    .summary =
        { $meshes ->
            [one] Ein Modell dieser Mod verwendet
           *[other] { $meshes } Modelle dieser Mod verwenden
        } für Farbe oder Oberfläche { $count ->
            [one] eine Textur
           *[other] { $count } Texturen
        }, die keine Mod und kein Archiv in deinem Spiel mitbringt: { $examples }.
    .explanation = Ein Modell ist die 3D-Form eines Dings; Farbe und feine Oberfläche bekommt es aus Texturen, eigenen Bilddateien, die das Modell nennt. Fehlt eine genannte Textur, malt das Spiel die Fläche grell lila, oder dunkel und flach, wo die Oberflächendetails fehlen. Meist ist dann ein Teil der Mod nicht installiert, zum Beispiel ihre Texturen in einem eigenen Download, oder die Mod gehört zu einer anderen Mod, die du nicht hast. Modelle, die eine Mod für eine Mod mitinstalliert, die du nicht verwendest, werden nie gezeigt; ihre fehlenden Texturen schaden dann nicht.
    .remedy = Sieh auf der Seite der Mod nach Dateien, die dazugehören, etwa einem Textur-Download oder einer Mod, die sie braucht, und installiere sie. Gehören die Modelle zu einem Patch für eine Mod, die du nicht hast, kannst du sie lassen.

meshes-retexture-bypassed = Textur-Mod erreicht ersetzte Modelle nicht
    .headline =
        { $textureMod } erreicht { $count ->
            [one] 1 Modell
           *[other] { $count } Modelle
        } { $meshOrigin ->
            [unmanaged] ohne Mod-Manager
           *[mod] von { $meshMod }
        } nicht
    .summary =
        { $textureMod } ersetzt Texturen, die von { $count ->
            [one] 1 Modell
           *[other] { $count } Modellen
        } { $meshOrigin ->
            [unmanaged] ohne Mod-Manager
           *[mod] aus { $meshMod }
        } nicht mehr verwendet werden. { $count ->
            [one] Das neue Modell verweist
           *[other] Die neuen Modelle verweisen
        } stattdessen auf andere Texturen. Zum Beispiel: { $examples }.
    .explanation = Ein Modell (Mesh) nennt die Texturen, die es verwendet, über ihren Pfad. Eine Textur-Mod wirkt, indem sie neue Dateien unter den Pfaden bereitstellt, die die ursprünglichen Modelle nennen. Ersetzt eine andere Mod ein Modell und nennt ihre Fassung andere Pfade, meist Texturen im eigenen Ordner dieser Mod, erreicht die Textur-Mod das Objekt nicht mehr. Das Objekt behält sein Aussehen, während alles drumherum das neue bekommt: eine helle Mauer neben dunklen, ein scharfes Dach neben unscharfen. Wie stark das auffällt, hängt davon ab, wie verschieden die beiden Textursätze sind.
    .remedy = Prüfe, ob die Textur-Mod eine Option oder einen Patch für die Modell-Mod anbietet; viele tun das. Andernfalls entscheide, welches Aussehen du willst: Entferne die genannten Modelle der Modell-Mod, damit die ursprünglichen Modelle und die Textur-Mod greifen, oder behalte sie und nimm den Unterschied in Kauf.

assets-loose-over-archive = Lose Dateien überdecken Dateien im Archiv einer anderen Mod
    .headline =
        { $winnerOrigin ->
            [unmanaged] Dateien ohne Mod-Manager überdecken
           *[mod] { $winnerMod } überdeckt
        } { $count ->
            [one] 1 Datei
           *[other] { $count } Dateien
        } von { $archiveMod }
    .summary =
        { $winnerOrigin ->
            [unmanaged] Dateien, die kein Mod-Manager installiert hat,
           *[mod] Lose Dateien von { $winnerMod }
        } ersetzen { $count ->
            [one] 1 Datei
           *[other] { $count } Dateien
        } in { $archive } von { $archiveMod }. Zum Beispiel: { $examples }.
    .explanation = Eine Mod kann ihre Dateien in ein Archiv gepackt oder als lose Dateien mitbringen. Das Spiel zieht eine lose Datei immer derselben Datei in einem Archiv vor, egal in welcher Reihenfolge die Mods stehen. Mod-Manager zeigen Konflikte zwischen losen Dateien, diese Art aber nicht, weil eine Seite im Archiv verborgen ist. Oft ist das gewollt, etwa bei einem Texturpaket über einer älteren Mod. Zum Problem wird es, wenn nur ein Teil zusammengehöriger Dateien ersetzt wird, etwa ein Modell ohne seine Textur: Dann erscheinen Objekte lila oder an der falschen Stelle.
    .remedy = Nichts zu tun, wenn die losen Dateien gewinnen sollen. Sieht die Mod aus dem Archiv im Spiel kaputt aus, deaktiviere die Mod mit den losen Dateien oder sieh auf ihrer Seite nach, ob beide zusammenpassen.

assets-archive-over-archive = Ein Archiv überdeckt Dateien im Archiv einer anderen Mod
    .headline =
        { $winnerOrigin ->
            [unmanaged] { $winnerArchive }
           *[mod] { $winnerMod }
        } überdeckt { $count ->
            [one] 1 Datei
           *[other] { $count } Dateien
        } von { $archiveMod }
    .summary =
        { $winnerOrigin ->
            [unmanaged] { $winnerArchive }
           *[mod] { $winnerArchive } von { $winnerMod }
        } wird später geladen und ersetzt { $count ->
            [one] 1 Datei
           *[other] { $count } Dateien
        } in { $archive } von { $archiveMod }. Zum Beispiel: { $examples }.
    .explanation = Archive werden in der Reihenfolge der Plugins geladen, zu denen sie gehören. Enthalten zwei Archive dieselbe Datei, verwendet das Spiel die aus dem später geladenen. Mod-Manager zeigen das nicht, weil beide Seiten in Archiven verborgen sind. Harmlos ist es, wenn die spätere Mod auf der früheren aufbauen soll. Ein Problem ist es, wenn das spätere Archiv ältere Kopien von Dateien enthält, die es mit der früheren Mod teilt.
    .remedy = Nichts zu tun, wenn die spätere Mod gewinnen soll. Andernfalls ändere die Ladereihenfolge der beiden Plugins so, dass das gewünschte Archiv zuletzt geladen wird.

setup-file-unreadable = Eine Datei des Setups konnte nicht gelesen werden
    .headline = { $file } ist nicht lesbar
    .summary = { $file } konnte nicht gelesen werden: { $reason }
    .explanation = Mod Mallet hat diese Datei übersprungen. Probleme, an denen sie beteiligt ist, werden deshalb nicht gemeldet. Ist die Datei beschädigt, kann auch das Spiel Schwierigkeiten mit ihr haben. Meist steckt ein beschädigter Download dahinter.
    .remedy = Installiere die Mod, zu der die Datei gehört, neu. Kommt das Spiel mit der Datei zurecht und Mod Mallet nicht, melde das bitte.

setup-load-order-unknown = Die Ladereihenfolge konnte nicht gelesen werden
    .headline = Die Ladereihenfolge ist nicht lesbar
    .summary = Plugins und Archive wurden nicht geprüft: { $reason }
    .explanation = Mod Mallet muss wissen, welche Plugins aktiv sind und in welcher Reihenfolge das Spiel sie lädt. Ohne das lässt sich nicht sagen, welche Dateien und Einträge gewinnen, deshalb sind diese Prüfungen nicht gelaufen. Die übrigen Prüfungen sind davon nicht betroffen.
    .remedy = Starte das Spiel oder deinen Mod-Manager einmal, damit die Liste der Plugins geschrieben wird, und scanne dann erneut. Ist das Spiel an einem ungewöhnlichen Ort installiert, gib Mod Mallet den Ort an.

crashes-recent = Das Spiel ist kürzlich abgestürzt
    .headline =
        { $count ->
            [one] 1 Absturz
           *[other] { $count } Abstürze
        } in den letzten { $days } Tagen, zuletzt am { $last }
    .summary =
        Ein Crash-Logger hat { $count ->
            [one] einen Absturz
           *[other] { $count } Abstürze
        } des Spiels in den letzten { $days } Tagen aufgezeichnet, zuletzt am { $last }. Wo das Spiel versagt hat: { $examples }.
    .explanation = Ein Crash-Logger ist eine kleine Erweiterung des Script Extenders, die bei jedem Absturz einen Bericht schreibt. Der Bericht sagt, in welcher Programmdatei das Spiel versagt hat, und nennt, was in der Nähe war: Figuren, Objekte, Modelle und die Mods, aus denen sie stammen. Ihn zu lesen ist eine Frage der Wahrscheinlichkeit, nicht der Gewissheit. Mod Mallet sieht sich die Berichte der letzten Wochen an und sagt, auf welche Mods sie hindeuten; ein Absturz im Spiel selbst, bei dem nichts von einer Mod in der Nähe war, wird gezählt, aber niemandem angelastet.
    .remedy = Sieh dir die übrigen Meldungen zu Abstürzen an, falls es welche gibt. Stürzt das Spiel immer wieder an derselben Stelle ab, merke dir, was du gerade getan hast; das hilft jedem, der der Sache nachgeht.

crashes-in-dll = Das Spiel ist in einer Programmdatei einer Mod abgestürzt
    .headline =
        { $file }: { $count ->
            [one] 1 Absturz
           *[other] { $count } Abstürze
        } darin in letzter Zeit
    .summary =
        { $count ->
            [one] Ein Absturz in letzter Zeit, am { $last }, geschah
           *[other] { $count } Abstürze in letzter Zeit, zuletzt am { $last }, geschahen
        } in { $file }, einer Programmdatei, die eine Mod dem Script Extender hinzufügt.
    .explanation = Manche Mods bringen eine eigene Programmdatei mit, eine DLL, die der Script Extender ins Spiel lädt. Stürzt das Spiel ab, während es den Code dieser Datei ausführt, ist sie die wahrscheinlichste Ursache: Sie passt vielleicht nicht zur Version des Spiels, verträgt sich nicht mit einer anderen solchen Datei oder hat einen Fehler. Sicher ist das nicht: Die Datei kann auch von etwas anderem kaputte Daten bekommen haben.
    .remedy = Sieh auf der Seite der Mod nach einem Update und nach bekannten Problemen mit deiner Spielversion. Hören die Abstürze nicht auf, deaktiviere die Mod eine Weile und sieh, ob sie dann ausbleiben.

crashes-mod-involved = Absturzberichte deuten auf eine Mod hin
    .headline =
        { $count ->
            [one] 1 Absturzbericht deutet
           *[other] { $count } Absturzberichte deuten
        } auf diese Mod hin
    .summary =
        { $count ->
            [one] Ein Absturzbericht aus letzter Zeit, vom { $last }, nennt
           *[other] { $count } Absturzberichte aus letzter Zeit, zuletzt vom { $last }, nennen
        } Dinge dieser Mod in der Nähe des Absturzes: { $examples }.
    .explanation = Absturzberichte zählen auf, was gerade in Gebrauch war, als das Spiel versagt hat: Figuren, Objekte, Modelle und Programmdateien auf dem Weg zum Absturz. Tauchen dort Dinge einer Mod auf, kann die Mod beteiligt sein, etwa durch ein kaputtes Modell oder eine Figur mit ungewöhnlichem Aufbau. Sie kann auch nur zufällig in der Nähe gewesen sein, vor allem an belebten Orten. Je mehr Berichte auf dieselbe Mod deuten, desto wahrscheinlicher ist sie beteiligt.
    .remedy = Deuten mehrere Berichte auf diese Mod, sieh auf ihrer Seite nach bekannten Abstürzen und Patches. Um sicherzugehen, deaktiviere sie eine Weile oder meide den oben genannten Ort oder Gegenstand und sieh, ob die Abstürze ausbleiben.
mods-collides-widely = Vorschlag: Eine Mod kollidiert an vielen Orten mit anderen Mods
    .headline =
        Vorschlag: { $mod } kollidiert an { $count ->
            [one] 1 Ort
           *[other] { $count } Orten
        } mit anderen Mods
    .summary =
        { $objects } der { $total } Objekte, die { $mod } in die Welt stellt, stehen in oder an Objekten { $modCount ->
            [one] einer anderen Mod
           *[other] von { $modCount } anderen Mods
        }, an { $count } Orten: { $others }. Die Zahl hinter jeder Mod sagt, an wie vielen Orten sie mit { $mod } zusammenstößt.
    .explanation = Das ist nur ein Vorschlag; kaputt ist nichts. Zwei Mods, die denselben Teil der Welt verändern, ohne voneinander zu wissen, stellen ihre Gebäude, Bäume und Zäune dorthin, wo die andere schon etwas hat. Ein paar solcher Orte sind normal und stehen einzeln unter den Kollisionen. Kollidiert eine Mod so oft, waren sie und die anderen Mods wahrscheinlich nicht dafür gemacht, zusammen benutzt zu werden.
    .remedy = Entscheide, welche Fassung dieser Orte dir lieber ist. Ist es die der anderen Mods, kannst du die Mod in deinem Mod-Manager ausschalten, oder sie behalten und in der Liste der Kollisionen an allen Orten auf einmal gegen die anderen Mods verlieren lassen. Gefällt dir die Mod, behalte sie und suche auf ihrer Seite nach einem Patch, der sie mit den anderen Mods verträglich macht.

mods-mostly-overridden = Vorschlag: Von einer Mod kommt kaum etwas im Spiel an
    .headline = Vorschlag: Nur { $percent } % von { $mod } kommen im Spiel an
    .summary = Von den { $total } Teilen von { $mod }, die Mod Mallet verglichen hat, kommen nur { $surviving } im Spiel an. Den Rest ersetzen Mods, die später geladen werden: { $overriders }. Die Zahl hinter jeder sagt, wie viele Teile sie ersetzt.
    .explanation = Das ist nur ein Vorschlag; kaputt ist nichts. Ändern zwei Mods dasselbe, nimmt das Spiel die Fassung der Mod, die es später lädt. Mod Mallet hat die Objekte verglichen, die die Mod in die Welt stellt, die Figuren, die sie ändert, und die Dateien in ihren Archiven. Eine Mod kann noch mehr mitbringen, etwa Quests, Gegenstände oder Skripte; die sind hier nicht mitgezählt.
    .remedy = Fehlt dir im Spiel nichts von dieser Mod, kannst du sie in deinem Mod-Manager ausschalten; das macht das Setup übersichtlicher und lädt schneller. Sieh vorher auf ihrer Seite nach: Manche Mods werden von anderen gebraucht, und die würden dann nicht mehr funktionieren. Ist dir ihre Fassung lieber, lade ihr Plugin nach den oben genannten Mods.

mods-duplicates-another = Vorschlag: Eine Mod stellt größtenteils dieselben Objekte auf wie eine andere
    .headline = Vorschlag: Behalte { $mod } oder { $other }, nicht beide
    .summary =
        { $count } der { $total } Objekte, die { $mod } in die Welt stellt, stellt auch { $other } auf, dasselbe Modell am selben Ort. { $patchCount ->
            [0] Für { $mod } allein ist keine andere Mod gemacht.
            [one] Für { $mod } allein ist ein Plugin gemacht: { $patches }.
           *[other] Für { $mod } allein sind { $patchCount } Plugins gemacht: { $patches }.
        } { $otherPatchCount ->
            [0] Für { $other } allein ist keine andere Mod gemacht.
            [one] Für { $other } allein ist ein Plugin gemacht: { $otherPatches }.
           *[other] Für { $other } allein sind { $otherPatchCount } Plugins gemacht: { $otherPatches }.
        }
    .explanation = Das ist nur ein Vorschlag; kaputt ist nichts. Sammlungen („all in one“) enthalten oft Mods, die es auch einzeln gibt, und zwei Mods können denselben Ort auf dieselbe Weise umbauen. Sind beide installiert, stehen dieselben Gebäude doppelt am selben Ort, und ihre Oberflächen flackern. Ein Plugin, das für eine der beiden gemacht ist, etwa ein Kompatibilitäts-Patch, funktioniert nur mit dieser, und das ist ein guter Grund, sie zu behalten.
    .remedy = Behalte die, die dir lieber ist, und schalte die andere in deinem Mod-Manager aus; nimm lieber die, für die die Patches gemacht sind, die du benutzt. Bevor du eine ausschaltest, sieh nach, ob sie etwas mitbringt, das der anderen fehlt, etwa einen Innenraum: Dann behalte beide und lass in der Liste der Kollisionen eine von ihnen an allen Orten auf einmal verlieren.
