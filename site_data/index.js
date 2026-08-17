

console.log("Connecting to game server...");

const proto = location.protocol === "https:" ? "wss:" : "ws:";
const ws = new WebSocket(`${proto}//${location.host}/websocket`);

let websocketCompleted = false;

const timer = setTimeout(() => {
    if (!websocketCompleted) {
        websocketCompleted = true;
        ws.close();
        console.log("Failed connecting to server! (timeout)");
    }
}, 1000);

const indexDiv = document.getElementById("index");
const errorDiv = document.getElementById("errors");

ws.onopen = () => {
    if (websocketCompleted) return;
    websocketCompleted = true;
    clearTimeout(timer);
    console.log("Connected to game server!");

    // Start listening only now that the handshake went through. The server sends
    // a SwitchPhase the moment it accepts us, but attaching here can't miss it:
    // the socket always dispatches `open` before any `message`.
    ws.onmessage = handleServerMessage;

    // From here on, a failure is a live connection dropping rather than a
    // handshake that never happened, so the two below replace the setup handler.
    ws.onerror = handleConnectionError;
    ws.onclose = handleConnectionClosed;
};

ws.onerror = () => {
    if (websocketCompleted) return;
    websocketCompleted = true;
    clearTimeout(timer);
    console.log(`Failed connecting to game server! Unknown error.`);
};

/// Something went wrong on an already-established connection. A `close` follows
/// this, so the cleanup belongs there, not here.
function handleConnectionError() {
    // TODO: surface to the user
    console.log("Game server connection errored.");
}

/// The connection is gone for good - the server drops it whenever the game ends
/// or the socket sends us something it considers unrecoverable.
function handleConnectionClosed(event) {
    // TODO: surface to the user, and decide whether to offer a reconnect
    console.log(`Disconnected from game server. (code ${event.code})`);
}

// --- Incoming: ServerMessage ------------------------------------------------
//
// Serde serializes these enums externally tagged, so a variant without data is
// just its name as a string, and one with data is a single-key object:
//
//   "InvalidJSON"
//   { "Welcome": { "your_id": number, "host_id": number, "your_pronouns": "Male" | "Female" | "Mixed" } }
//   { "InvalidRequest": { "reason": "..." } }
//   { "InternalServerError": { "err": "..." } }
//   { "SwitchPhase": <PublicGamePhase> }
//
// PublicGamePhase is tagged the same way:
//
//   { "Setup":               { "connected_players": [Player] } }
//   { "SelectPresenter":     { "possible_presenters": [Player],
//                              "skip_percentage": number } }
//   { "CurrentlyPresenting": { "presentation": Presentation,
//                              "presentation_user": Player,
//                              "players_who_skipped": [Player],
//                              "total_players": number,
//                              "skip_percentage": number } }
//   { "Results":             { "all_presentations": [Presentation] } }
//
// Player      = { id: number, name: string,
//                 presentation_title: string | null,
//                 pronouns: "Male" | "Female" | "Mixed" }
// Presentation = { presenter_id: number, name: string,
//                  start_time: string, end_time: string }  // RFC 3339 UTC

/// Splits an externally tagged enum into its variant name and payload.
/// The payload is null for variants that carry no data.
function unwrapTaggedEnum(message) {
    if (typeof message === "string") {
        return { variant: message, payload: null };
    }

    const keys = Object.keys(message);

    if (keys.length !== 1) {
        return null;
    }

    return { variant: keys[0], payload: message[keys[0]] };
}

// --- Who am I ---------------------------------------------------------------
//
// Three UIs to draw: the host's, the current presenter's, and everyone else's.
// The server hands us the two ids we can't work out on our own in `Welcome`,
// which always lands before the first phase. Everything else is derived off the
// ids already inside the phase payloads, so there's no role state to keep in
// sync as the game moves.
//
// None of this is a permission check - the server rechecks every request that
// matters. Getting it wrong here just draws a button the server will refuse.

let me = { id: null, hostId: null, pronouns: null };

/// The player hosting the game. Fixed for the whole game.
function amHost() {
    return me.id !== null && me.id === me.hostId;
}

/// The player whose presentation is on screen right now. Changes every phase, so
/// this takes the presentation being rendered rather than reading global state.
function amPresenter(presentation) {
    return me.id !== null && presentation.presenter_id === me.id;
}

/// Is this one of the players in the list us? For highlighting ourselves in
/// player lists.
function isMe(player) {
    return me.id !== null && player.id === me.id;
}

/// The server telling us who we are. First thing it sends, once per connection.
function handleWelcome(yourId, hostId, your_pronouns) {
    me = { id: yourId, hostId: hostId, pronouns: your_pronouns };
    console.log(`We are player ${yourId}, with pronouns ${your_pronouns}. Host is ${hostId}.`, amHost() ? "(that's us)" : "");
}

/// Entry point for everything the server sends us. Attached in `ws.onopen`.
function handleServerMessage(event) {
    let serverMessage;

    try {
        serverMessage = JSON.parse(event.data);
    } catch (err) {
        console.log(`Received unparseable message from server: ${event.data}`);
        return;
    }

    const tagged = unwrapTaggedEnum(serverMessage);

    if (!tagged) {
        console.log(`Received malformed ServerMessage: ${event.data}`);
        return;
    }

    switch (tagged.variant) {
        case "Welcome":
            handleWelcome(tagged.payload.your_id, tagged.payload.host_id, tagged.payload.your_pronouns);
            break;
        case "SwitchPhase":
            handleSwitchPhase(tagged.payload);
            break;
        case "InvalidJSON":
            handleInvalidJSON();
            break;
        case "InvalidRequest":
            handleInvalidRequest(tagged.payload.reason);
            break;
        case "InternalServerError":
            handleInternalServerError(tagged.payload.err);
            break;
        default:
            console.log(`Unknown ServerMessage variant: ${tagged.variant}`);
    }
}

/// The game moved to a new phase; redraw the page to match it.
function handleSwitchPhase(publicGamePhase) {
    const tagged = unwrapTaggedEnum(publicGamePhase);

    if (!tagged) {
        console.log(`Received malformed PublicGamePhase: ${JSON.stringify(publicGamePhase)}`);
        return;
    }

    // First, clean index. The clock, if one is running, points at an element
    // that's about to be thrown away, so it has to go with it.
    stopCountdown();
    indexDiv.textContent = '';

    switch (tagged.variant) {
        case "Setup":
            renderSetup(tagged.payload.connected_players);
            break;
        case "SelectPresenter":
            renderSelectPresenter(tagged.payload.possible_presenters, tagged.payload.skip_percentage);
            break;
        case "CurrentlyPresenting":
            renderCurrentlyPresenting(
                tagged.payload.presentation,
                tagged.payload.presentation_user,
                tagged.payload.players_who_skipped,
                tagged.payload.total_players,
                tagged.payload.skip_percentage
            );
            break;
        case "Results":
            renderResults(tagged.payload.all_presentations);
            break;
        default:
            console.log(`Unknown PublicGamePhase variant: ${tagged.variant}`);
    }
}

/// The game hasn't started yet and players are logging in.
function renderSetup(connectedPlayers) {
    console.log("Phase: Setup", connectedPlayers);

    const title = document.createElement("h1");
    title.innerText = "ברוכים הבאים ליום ההולדת של אמיר!";

    const subtitle = document.createElement("h2");
    subtitle.innerText = "עוד מעט נתחיל...";

    const currentPlayerDiv = document.createElement("div");
    currentPlayerDiv.innerText = "כרגע מחוברים: " + connectedPlayers.map((player) => player.name).join(", ");

    indexDiv.append(title, subtitle, currentPlayerDiv);


    if (amHost()) {
        const startGameButton = document.createElement("button");
        startGameButton.innerText = insertRelevantPronouns("[התחל] משחק");
        startGameButton.onclick = sendStartGame;
        indexDiv.appendChild(startGameButton);
    }

}

/// The host needs to select who presents next.
function renderSelectPresenter(possiblePresenters, skipPercentage) {
    console.log("Phase: SelectPresenter", possiblePresenters);

    if (!amHost()) {
        const titleDiv = document.createElement("h1");
        titleDiv.innerText = "בוחרים את המצגת הבאה!";

        const textDiv = document.createElement("p");
        textDiv.innerText = insertRelevantPronouns("אנא [חכה] שהמצגת תיבחר");

        indexDiv.append(titleDiv, textDiv);
    }

    else {
        const titleDiv = document.createElement("h1");
        titleDiv.innerText = insertRelevantPronouns("[בחר] את המצגת הבאה!");

        const presentationButtons = possiblePresenters.map((player) => {
            const playerButton = document.createElement("button");
            playerButton.innerText = `${player.name}: ${player.presentation_title}`;
            playerButton.onclick = () => sendSelectPresenter(player.id);

            return playerButton;
        });

        indexDiv.append(titleDiv, ...presentationButtons);
        indexDiv.appendChild(createSkipPercentageBar(skipPercentage));
    }
}

/// Someone is presenting right now.
function renderCurrentlyPresenting(presentation, presentationUser, playersWhoSkipped, totalPlayers, skipPercentage) {
    console.log("Phase: CurrentlyPresenting", presentation, playersWhoSkipped, totalPlayers);

    const countdownElement = document.createElement("span");
    startCountdown(countdownElement, presentation.start_time);

    const countdownTextWrapper = document.createElement("p");
    countdownTextWrapper.innerText = "אורך מצגת נוכחי: ";

    const countdownClockDiv = document.createElement("div");
    countdownClockDiv.append(countdownTextWrapper, countdownElement);

    const skipPresentationButton = document.createElement("button");
    skipPresentationButton.type = "button";
    skipPresentationButton.innerText = insertRelevantPronouns("[סיים] את המצגת!");
    skipPresentationButton.onclick = sendEndPresentation;

    if (amPresenter(presentation)) {
        const stopLookingText = document.createElement("h1");
        stopLookingText.innerText = insertRelevantPronouns("[תעיף] את הראש מהטלפון!");

        const subtitleText = document.createElement("h2");
        subtitleText.innerText = "אנשים מקשיבים למצגת שלך!"

        const shutUpAlreadyCounter = document.createElement("p");
        if (playersWhoSkipped.length == 0) {
            shutUpAlreadyCounter.innerText = `בנתיים לא הצביעו להעביר את המצגת.`;
        } else if (playersWhoSkipped.length == 1) {
            shutUpAlreadyCounter.innerText = `מישהו הצביע להעביר את המצגת.`;
        } else {
            shutUpAlreadyCounter.innerText = `${playersWhoSkipped.length} אנשים הצביעו להעביר את המצגת.`;
        }

        indexDiv.append(stopLookingText, subtitleText, countdownClockDiv, shutUpAlreadyCounter, skipPresentationButton);
    } else {
        const skipButton = document.createElement("button");
        skipButton.type = "button";
        skipButton.classList.add("skipButton");

        if (playersWhoSkipped.some((player) => isMe(player))) {
            skipButton.classList.add("active");
            skipButton.innerText = insertRelevantPronouns("האמת, אולי יש [לו] משהו להגיד...", presentationUser.pronouns);
            skipButton.onclick = () => sendSkipDecision(false);
        } else {
            skipButton.innerText = insertRelevantPronouns("[תגיד] ") + insertRelevantPronouns("[לו] ש[ישתוק]!", presentationUser.pronouns);
            skipButton.onclick = () => sendSkipDecision(true);
        }


        const totalSkippersDiv = document.createElement("div");
        totalSkippersDiv.innerText = `כרגע ${playersWhoSkipped.length} מתוך ${totalPlayers} רוצים להסיים את המצגת`;

        const presentationTitle = document.createElement("h1");
        presentationTitle.innerText = presentation.name;


        indexDiv.append(presentationTitle, totalSkippersDiv, skipButton);

        if (amHost()) {
            indexDiv.appendChild(createSkipPercentageBar(skipPercentage));
            indexDiv.appendChild(skipPresentationButton);
        }
    }
}

/// The game is over and we're showing off what happened.
function renderResults(allPresentations) {
    // TODO
    console.log("Phase: Results", allPresentations);
}

/// We sent the server something it couldn't parse. The server closes the socket
/// after this one.
function handleInvalidJSON() {
    // TODO: surface to the user
    console.log("Server rejected our message as invalid JSON.");
}

/// We asked for something we're not allowed to do, or that makes no sense in
/// this phase.
function handleInvalidRequest(reason) {
    // TODO: surface to the user
    console.log(`Invalid request: ${reason}`);
}

/// The server broke, not us.
function handleInternalServerError(err) {
    // TODO: surface to the user
    console.log(`Internal server error: ${err}`);
}

// Returns an HTML object for the host, which is a bar that allows them to send changes to the minimal percent to skip the presentation.
function createSkipPercentageBar(currentSkipPercentage = 0.8) {
    const skipPercentageBarSection = document.createElement("div");

    const rangeBar = document.createElement("input");
    rangeBar.type = "range";
    rangeBar.min = 0;
    rangeBar.max = 1;
    rangeBar.step = 0.01;
    rangeBar.defaultValue = currentSkipPercentage;
    rangeBar.onchange = (event) => sendChangeSkipPercentage(event.target.value);

    const textExplanation = document.createElement("p");
    textExplanation.innerText = `Change the bar to change the percentage required for a presentation to be skipped. Currently at: ${currentSkipPercentage * 100}%`;

    skipPercentageBarSection.append(rangeBar, textExplanation);

    return skipPercentageBarSection;
}

// --- Outgoing: UserMessage --------------------------------------------------
//
// Same tagging rules as above, going the other way.

function sendUserMessage(message) {
    if (ws.readyState !== WebSocket.OPEN) {
        console.log(`Tried sending while socket wasn't open: ${JSON.stringify(message)}`);
        return;
    }

    ws.send(JSON.stringify(message));
}

/// Host only. Starts the game.
function sendStartGame() {
    sendUserMessage("StartGame");
}

/// Host only. Picks who presents next.
function sendSelectPresenter(presenterId) {
    sendUserMessage({ SelectPresenter: { presenter_id: presenterId } });
}

/// Host or presenter only. Ends the presentation currently running.
function sendEndPresentation() {
    sendUserMessage("EndPresentation");
}

/// Anyone. Changes the stance on presentation skipping; doesn't skip it on its own.
function sendSkipDecision(whetherToSkip = true) {
    sendUserMessage({ ChangePresentationSkipStance: { want_to_skip: whetherToSkip } });
}

/// Host only. `newPercentage` is a fraction, not a percent - 0.75 means 75%.
function sendChangeSkipPercentage(newPercentage) {
    sendUserMessage({ ChangeSkipPercentage: { new_percentage: parseFloat(newPercentage) } });
}


/// How long a presentation ran, in whole seconds. Takes the two RFC 3339 strings
/// the server puts on a Presentation. Returns null if either one doesn't parse.
///
/// Note that a presentation still in progress carries a placeholder `end_time`,
/// so this is only meaningful once the presentation has actually ended.
function presentationDurationInSeconds(startTime, endTime) {
    const start = Date.parse(startTime);
    const end = Date.parse(endTime);

    if (Number.isNaN(start) || Number.isNaN(end)) {
        console.log(`Couldn't parse presentation times: "${startTime}" - "${endTime}"`);
        return null;
    }

    return Math.round((end - start) / 1000);
}

/// Seconds as MM:SS. Minutes keep counting up past two digits rather than
/// wrapping, so an hour-long presentation reads 63:07 instead of starting over.
function formatAsMinutesAndSeconds(totalSeconds) {
    const minutes = Math.floor(totalSeconds / 60);
    const seconds = totalSeconds % 60;

    return `${String(minutes).padStart(2, "0")}:${String(seconds).padStart(2, "0")}`;
}

/// The interval driving the on-screen presentation clock, if one is running.
/// Only ever one at a time - a phase switch stops the old one before the new
/// render gets a chance to start another.
let countdownIntervalId = null;

/// Updates `element` once a second with how long it's been since `startTime`,
/// the RFC 3339 string the server puts on a Presentation.
function startCountdown(element, startTime) {
    const start = Date.parse(startTime);

    if (Number.isNaN(start)) {
        console.log(`Couldn't parse presentation start time: "${startTime}"`);
        element.innerText = "--:--";
        return;
    }

    const tick = () => {
        // Our clock and the server's don't have to agree. If ours is behind, the
        // elapsed time comes out negative for the first seconds of a
        // presentation, and 00:00 is a friendlier lie than -00:03.
        const elapsedSeconds = Math.max(0, Math.floor((Date.now() - start) / 1000));
        element.innerText = formatAsMinutesAndSeconds(elapsedSeconds);
    };

    stopCountdown();
    tick(); // The first interval tick is a second away; don't show an empty clock until then.
    countdownIntervalId = setInterval(tick, 1000);
}

/// Stops the running clock, if there is one. Safe to call when there isn't.
function stopCountdown() {
    if (countdownIntervalId === null) {
        return;
    }

    clearInterval(countdownIntervalId);
    countdownIntervalId = null;
}

/// Given a string, returns one where all the gendered words marked with [] have been replaced with the appropriate ones from this dictionary.
function insertRelevantPronouns(phrase, userPronouns = me.pronouns) {
    return phrase.replace(/\[([^\]\n]+)\]/g, (_, word) => translateWord(word));


    /// Assumes the passed string is male gendered. If the string has no replacement, returns ""
    function translateWord(genderedWord) {
        // For convenience, mixed are 0 index, male are 1, female are 2
        let pronounIndex = 0;
        if (userPronouns == "Male") {
            pronounIndex = 1;
        } else if (userPronouns == "Female") {
            pronounIndex = 2;
        }

        const translationDictionary = {
            "חכה": ["חכה.י", "חכה", "חכי"],
            "התחל": ["התחל.י", "התחל", "התחלי"],
            "מובן": ["מוכנ.ה", "מוכן", "מוכנה"],
            "בחר": ["בחר.י", "בחר", "בחרי"],
            "תעיף": ["תעיפ.י", "תעיף", "תעיפי"],
            "תגיד": ["תגיד.י", "תגיד", "תגידי"],
            "לו": ["להם", "לו", "לה"],
            "ישתוק": ["ת.ישתוק", "ישתוק", "תשתוק"],
            "סיים": ["סיימ.י", "סיים", "סיימי"]
        }

        const relevantWord = translationDictionary[genderedWord];
        if (!relevantWord) {
            return "";
        } else {
            return relevantWord[pronounIndex];
        }
    }
}