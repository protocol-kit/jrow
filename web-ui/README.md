# JROW Web Client

A vanilla HTML/JavaScript web-based client for testing and interacting with JROW (JSON-RPC over WebSocket) servers.

## Features

- ✅ **Zero build required** - Just open in browser
- ✅ **WebSocket connection management** - Connect/disconnect with status indicator
- ✅ **Request/Response** - Send JSON-RPC requests and view responses
- ✅ **Notifications** - Send one-way notifications
- ✅ **Pub/Sub subscriptions** - Subscribe to topics with pattern support
- ✅ **Batch requests** - Send multiple requests at once
- ✅ **Server Logs** - View real-time server logs pushed from the server
- ✅ **Real-time console** - View all WebSocket messages
- ✅ **Beautiful UI** - Modern dark theme design
- ✅ **Keyboard shortcuts** - Ctrl+Enter to send

## Quick Start

### Option 1: Run Server with Embedded UI (Recommended)

The easiest way - runs both JROW server and web UI with one command:

```bash
# From project root
make run-server-ui

# Or directly:
cargo run --example server_with_ui

# Open http://127.0.0.1:8080 in your browser
# UI automatically connects to ws://127.0.0.1:8081
```

**This is the easiest option!** Everything is configured and ready to test.

### Option 2: Open Directly (No Server)

If you have a JROW server running and just want to connect:

```bash
# Open index.html in your browser
open web-ui/index.html

# Or on Linux
xdg-open web-ui/index.html

# Or on Windows
start web-ui/index.html
```

**Note:** Some browsers may restrict WebSocket connections from `file://` URLs. If you have issues, use Option 3 or 4.

### Option 3: Python HTTP Server

```bash
cd web-ui
python3 -m http.server 8000
```

Then open http://localhost:8000 in your browser.

### Option 4: Node.js HTTP Server

```bash
cd web-ui
npx serve
```

Then open http://localhost:3000 in your browser.

### Option 5: Make Command (from project root)

```bash
make run-web-ui
```

## Usage Guide

### 1. Connect to Server

1. Enter your JROW server WebSocket URL (e.g., `ws://localhost:8080`)
2. Click **Connect**
3. Watch the status indicator turn green

### 2. Send a Request

Go to the **Request** tab:

1. Enter method name (e.g., `echo`, `add`, `getUserProfile`)
2. Enter parameters as JSON (e.g., `{"message": "Hello"}`)
3. Click **Send Request** or press `Ctrl+Enter`
4. View the response below

**Example:**
```json
Method: echo
Params: {"message": "Hello JROW!"}
```

### 3. Send a Notification

Go to the **Notify** tab:

1. Enter method name
2. Enter parameters as JSON
3. Click **Send Notification**

**Note:** Notifications don't expect a response.

### 4. Subscribe to Topics

Go to the **Subscribe** tab:

1. Enter topic name or pattern (e.g., `events.*`, `stock.prices.>`)
2. Click **Subscribe**
3. Watch messages appear in real-time below

**Pattern Examples:**
- `events.*` - Matches `events.user`, `events.admin`
- `events.>` - Matches `events.user.login`, `events.user.logout`, etc.
- `stock.prices.*` - Matches all stock tickers

**Unsubscribe:**
- Enter topic name and click **Unsubscribe**, or
- Click the **✖** button next to the topic in Active Subscriptions

### 5. Send Batch Requests

Go to the **Batch** tab:

1. Edit the JSON array of requests
2. Mix requests and notifications
3. Click **Send Batch**
4. View responses (or check Console tab)

**Example:**
```json
[
  {
    "jsonrpc": "2.0",
    "method": "add",
    "params": {"a": 5, "b": 3},
    "id": 1
  },
  {
    "jsonrpc": "2.0",
    "method": "multiply",
    "params": {"a": 2, "b": 4},
    "id": 2
  }
]
```

### 6. View Server Logs

Go to the **Server Logs** tab:

1. Click **Subscribe to Logs** to start receiving server logs
2. Watch real-time logs appear from the server
3. See log levels: INFO, SUCCESS, WARNING, ERROR, DEBUG
4. Click **Clear Logs** to reset the view
5. Click **Unsubscribe** to stop receiving logs

**What you'll see:**
- Server health checks
- Background task completion
- Connection events
- System resource usage
- Request processing activity

**Note:** The server must publish to the `server.logs` topic for this feature to work. The `server_with_ui` example automatically does this.

### 7. View Message Log

Go to the **Console** tab:

- See all sent and received messages
- Color-coded by message type
- Toggle timestamps and auto-scroll
- Click **Clear Console** to reset

## Testing with JROW Examples

### Simple Server Example

```bash
# Terminal 1: Start JROW server
cargo run --example simple_server

# Terminal 2: Start web UI (if using HTTP server)
cd web-ui
python3 -m http.server 8000

# Open browser to http://localhost:8000
# Connect to ws://localhost:8080
```

### Pub/Sub Example

```bash
# Terminal 1: Run pub/sub example
cargo run --example pubsub

# The example starts a server on ws://localhost:8081
# Connect the web UI to ws://localhost:8081
# Subscribe to: stock.prices.*
# Watch messages arrive!
```

### Test Different Methods

Most JROW examples support these methods:

**Simple Server (port 8080):**
- `echo` - Echo back params
- `add` - Add two numbers: `{"a": 5, "b": 3}`
- `multiply` - Multiply: `{"a": 2, "b": 4}`

**Pub/Sub (port 8081):**
- Subscribe to: `stock.prices.*`
- Subscribe to: `news.>` 
- Subscribe to: `alerts.critical`

## Keyboard Shortcuts

- `Ctrl+Enter` (or `Cmd+Enter` on Mac) - Send request/notification/batch from current tab
- Tab through form fields for quick navigation

## Browser Compatibility

Works in all modern browsers:
- ✅ Chrome/Edge (recommended)
- ✅ Firefox
- ✅ Safari
- ✅ Opera

**Requirements:**
- WebSocket support (available in all modern browsers)
- JavaScript enabled

## Troubleshooting

### "Connection failed"

**Check:**
1. Is the JROW server running?
2. Is the WebSocket URL correct? (should start with `ws://` or `wss://`)
3. Is the port correct?
4. Check server logs for errors

**Test server:**
```bash
cargo run --example simple_server
# Should see: "Starting JROW server on 127.0.0.1:8080"
```

### "Not connected to server"

Click the **Connect** button first before sending requests.

### CORS or Mixed Content Errors

- If serving web UI over HTTPS, connect to `wss://` (not `ws://`)
- Use same protocol (HTTP with WS, HTTPS with WSS)

### WebSocket blocked on `file://`

Some browsers restrict WebSocket from file URLs. Use HTTP server:
```bash
python3 -m http.server 8000
```

## Files

- `index.html` - Main UI interface
- `styles.css` - Styling and responsive design
- `jrow-client.js` - JROW WebSocket client library
- `app.js` - Application logic and event handlers
- `README.md` - This file

## Customization

### Change Default URL

Edit `index.html`, line ~30:
```html
<input type="text" id="wsUrl" value="ws://YOUR_SERVER:PORT">
```

### Modify UI Colors

Edit `styles.css`, `:root` section:
```css
:root {
    --primary-color: #007bff;  /* Change primary color */
    --success-color: #28a745;  /* Change success color */
    /* ... */
}
```

### Add Custom Methods

The web UI works with any JSON-RPC methods your server supports. Just type the method name in the Request tab.

## Using the JROW Client Library

The `jrow-client.js` file can be used in your own projects:

```html
<script src="jrow-client.js"></script>
<script>
  const client = new JrowClient();
  
  // Connect
  await client.connect('ws://localhost:8080');
  
  // Send request
  const result = await client.request('echo', { msg: 'Hello' });
  console.log(result);
  
  // Subscribe
  await client.subscribe('events.*', (data) => {
    console.log('Received:', data);
  });
  
  // Disconnect
  client.disconnect();
</script>
```

See `jrow-client.js` for full API documentation.

## Development

To modify the web UI:

1. Edit the HTML/CSS/JS files
2. Refresh browser (no build step!)
3. Test with JROW examples

The code is intentionally simple and dependency-free for easy customization.

## License

This web client is part of the JROW project and is dual-licensed:
- Code: MIT-0 (do whatever you want)
- Non-code: CC0-1.0 (public domain)

## Related Documentation

- [JROW Main Documentation](../README.md)
- [JROW Specification](../docs/SPECIFICATION.md)
- [Use Cases Guide](../docs/use-cases.md)
- [Examples Directory](../examples/)

## Feedback

Found a bug or have a feature request? Open an issue on GitHub!

