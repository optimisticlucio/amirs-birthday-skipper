

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
//   { "SelectPresenter":     { "possible_presenters": [Player] } }
//   { "CurrentlyPresenting": { "presentation": Presentation,
//                              "players_who_skipped": [Player],
//                              "total_players": number } }
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
    me = { id: yourId, hostId: hostId, your_pronouns: your_pronouns };
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

    // First, clean index.
    indexDiv.textContent = '';

    switch (tagged.variant) {
        case "Setup":
            renderSetup(tagged.payload.connected_players);
            break;
        case "SelectPresenter":
            renderSelectPresenter(tagged.payload.possible_presenters);
            break;
        case "CurrentlyPresenting":
            renderCurrentlyPresenting(
                tagged.payload.presentation,
                tagged.payload.players_who_skipped,
                tagged.payload.total_players,
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
        startGameButton.innerText = "התחל משחק";
        startGameButton.onclick = sendStartGame;
        indexDiv.appendChild(startGameButton);
    }

}

/// The host needs to select who presents next.
function renderSelectPresenter(possiblePresenters) {
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
    }
}

/// Someone is presenting right now.
function renderCurrentlyPresenting(presentation, playersWhoSkipped, totalPlayers) {
    // TODO
    console.log("Phase: CurrentlyPresenting", presentation, playersWhoSkipped, totalPlayers);
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

/// Anyone. Votes to skip the current presentation; doesn't skip it on its own.
function sendVoteToSkipPresentation() {
    sendUserMessage("VoteToSkipPresentation");
}

/// Host only. `newPercentage` is a fraction, not a percent - 0.75 means 75%.
function sendChangeSkipPercentage(newPercentage) {
    sendUserMessage({ ChangeSkipPercentage: { new_percentage: newPercentage } });
}


/// Given a string, returns one where all the gendered words marked with [] have been replaced with the appropriate ones from this dictionary.
function insertRelevantPronouns(phrase) {
    return phrase.replace(/\[([^\]\n]+)\]/g, (_, word) => translateWord(word));


    /// Assumes the passed string is male gendered. If the string has no replacement, returns ""
    function translateWord(genderedWord) {

        console.log(`word passed is ${genderedWord}`)
        // For convenience, mixed are 0 index, male are 1, female are 2
        const pronounIndex = 0;
        if (self.pronouns == "Male") {
            pronounIndex = 1;
        } else if (self.pronouns == "Female") {
            pronounIndex = 2;
        }

        const translationDictionary = {
            "חכה": ["חכה.י", "חכה", "חכי"],
            "התחל": ["התחל.י", "התחל", "התחלי"],
            "מובן": ["מוכנ.ה", "מוכן", "מוכנה"],
            "בחר": ["בחר.י", "בחר", "בחרי"]
        }

        const relevantWord = translationDictionary[genderedWord];
        if (!relevantWord) {
            return "";
        } else {
            return relevantWord[pronounIndex];
        }
    }
}