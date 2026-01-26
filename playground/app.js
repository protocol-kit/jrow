/**
 * JROW Web Client Application
 * 
 * UI logic and event handlers for the JROW web client
 */

// Global client instance
let client = new JrowClient();

// Override the log method to write to console panel
client.log = function(message, type) {
    const showTimestamps = document.getElementById('showTimestamps')?.checked ?? true;
    const timestamp = showTimestamps ? `[${new Date().toLocaleTimeString()}] ` : '';
    
    addConsoleEntry(timestamp + message, type);
};

// Server logs tracking
let serverLogsSubscribed = false;
let serverLogsCount = 0;

// Initialize on page load
document.addEventListener('DOMContentLoaded', () => {
    // Set up connection event handlers
    client.on('open', () => {
        updateStatus('connected', 'Connected');
    });

    client.on('close', () => {
        updateStatus('disconnected', 'Disconnected');
        // Clear active subscriptions display
        updateSubscriptionsList();
        // Update server logs status
        serverLogsSubscribed = false;
        updateServerLogsStatus();
    });

    client.on('error', () => {
        updateStatus('disconnected', 'Connection Error');
    });

    // Log welcome message
    addConsoleEntry('JROW Web Client ready. Connect to a server to begin.', 'info');
});

// Connection Functions
async function connect() {
    const url = document.getElementById('wsUrl').value.trim();
    
    if (!url) {
        alert('Please enter a WebSocket URL');
        return;
    }

    if (!url.startsWith('ws://') && !url.startsWith('wss://')) {
        alert('URL must start with ws:// or wss://');
        return;
    }

    updateStatus('connecting', 'Connecting...');

    try {
        await client.connect(url);
    } catch (error) {
        updateStatus('disconnected', 'Connection Failed');
        alert('Connection failed: ' + error.message);
    }
}

function disconnect() {
    if (client.isConnected()) {
        client.disconnect();
    }
}

function updateStatus(className, text) {
    const status = document.getElementById('status');
    status.className = 'status ' + className;
    status.querySelector('.status-text').textContent = text;
}

// Tab Switching
function switchTab(tabName) {
    // Hide all panels
    document.querySelectorAll('.panel').forEach(panel => {
        panel.classList.remove('active');
    });
    
    // Remove active class from all tabs
    document.querySelectorAll('.tab').forEach(tab => {
        tab.classList.remove('active');
    });
    
    // Show selected panel
    document.getElementById(tabName).classList.add('active');
    
    // Add active class to clicked tab
    event.target.closest('.tab').classList.add('active');
}

// Request Functions
async function sendRequest() {
    const method = document.getElementById('reqMethod').value.trim();
    const paramsText = document.getElementById('reqParams').value.trim();
    const responseBox = document.getElementById('reqResponse');
    
    if (!client.isConnected()) {
        responseBox.textContent = 'Error: Not connected to server';
        return;
    }

    if (!method) {
        responseBox.textContent = 'Error: Method name is required';
        return;
    }

    let params = null;
    
    // Parse params if provided
    if (paramsText) {
        try {
            params = JSON.parse(paramsText);
        } catch (error) {
            responseBox.textContent = 'Error: Invalid JSON in params\n' + error.message;
            return;
        }
    }

    responseBox.textContent = 'Sending request...';

    try {
        const result = await client.request(method, params);
        responseBox.textContent = JSON.stringify(result, null, 2);
    } catch (error) {
        responseBox.textContent = 'Error: ' + JSON.stringify(error, null, 2);
    }
}

// Notification Functions
async function sendNotification() {
    const method = document.getElementById('notifyMethod').value.trim();
    const paramsText = document.getElementById('notifyParams').value.trim();
    
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (!method) {
        alert('Method name is required');
        return;
    }

    let params = null;
    
    if (paramsText) {
        try {
            params = JSON.parse(paramsText);
        } catch (error) {
            alert('Invalid JSON in params: ' + error.message);
            return;
        }
    }

    try {
        client.notify(method, params);
        alert('Notification sent (no response expected)');
    } catch (error) {
        alert('Error: ' + error.message);
    }
}

// Subscription Functions
async function subscribe() {
    const topic = document.getElementById('subTopic').value.trim();
    
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (!topic) {
        alert('Topic name is required');
        return;
    }

    // Check if already subscribed
    if (client.getSubscriptions().includes(topic)) {
        alert('Already subscribed to: ' + topic);
        return;
    }

    try {
        await client.subscribe(topic, (data) => {
            addSubscriptionMessage(topic, data);
        });
        
        updateSubscriptionsList();
        alert('Subscribed to: ' + topic);
    } catch (error) {
        alert('Subscribe failed: ' + JSON.stringify(error));
    }
}

async function unsubscribe() {
    const topic = document.getElementById('subTopic').value.trim();
    
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (!topic) {
        alert('Topic name is required');
        return;
    }

    try {
        await client.unsubscribe(topic);
        updateSubscriptionsList();
        alert('Unsubscribed from: ' + topic);
    } catch (error) {
        alert('Unsubscribe failed: ' + JSON.stringify(error));
    }
}

async function unsubscribeFromTopic(topic) {
    try {
        await client.unsubscribe(topic);
        updateSubscriptionsList();
    } catch (error) {
        alert('Unsubscribe failed: ' + JSON.stringify(error));
    }
}

function updateSubscriptionsList() {
    const container = document.getElementById('activeSubscriptions');
    const subscriptions = client.getSubscriptions();
    
    if (subscriptions.length === 0) {
        container.innerHTML = '<em class="empty-state">No active subscriptions</em>';
        return;
    }
    
    container.innerHTML = subscriptions.map(topic => `
        <div class="subscription-item">
            <span>📡 ${escapeHtml(topic)}</span>
            <button onclick="unsubscribeFromTopic('${escapeHtml(topic)}')" title="Unsubscribe">✖</button>
        </div>
    `).join('');
}

function addSubscriptionMessage(topic, data) {
    const container = document.getElementById('subMessages');
    const timestamp = new Date().toLocaleTimeString();
    
    const messageDiv = document.createElement('div');
    messageDiv.className = 'message';
    
    const timeDiv = document.createElement('div');
    timeDiv.className = 'message-time';
    timeDiv.textContent = timestamp;
    
    const topicDiv = document.createElement('div');
    topicDiv.className = 'message-topic';
    topicDiv.textContent = '📡 ' + topic;
    
    const dataDiv = document.createElement('div');
    dataDiv.className = 'message-data';
    dataDiv.textContent = JSON.stringify(data, null, 2);
    
    messageDiv.appendChild(timeDiv);
    messageDiv.appendChild(topicDiv);
    messageDiv.appendChild(dataDiv);
    
    container.appendChild(messageDiv);
    container.scrollTop = container.scrollHeight;
}

function clearSubscriptionMessages() {
    document.getElementById('subMessages').innerHTML = '';
}

// Batch Functions
async function sendBatch() {
    const batchText = document.getElementById('batchJson').value.trim();
    const responseBox = document.getElementById('batchResponse');
    
    if (!client.isConnected()) {
        responseBox.textContent = 'Error: Not connected to server';
        return;
    }

    let batch;
    try {
        batch = JSON.parse(batchText);
    } catch (error) {
        responseBox.textContent = 'Error: Invalid JSON\n' + error.message;
        return;
    }

    if (!Array.isArray(batch)) {
        responseBox.textContent = 'Error: Batch must be a JSON array';
        return;
    }

    responseBox.textContent = 'Sending batch...';

    try {
        const startTime = Date.now();
        
        // Send the batch as-is to the server
        client.sendBatch(batch);
        
        // Count requests vs notifications
        const requests = batch.filter(item => item.id !== undefined);
        const notifications = batch.filter(item => item.id === undefined);
        
        if (requests.length === 0) {
            // All notifications, no responses expected
            responseBox.textContent = `✅ Batch sent! (${notifications.length} notifications)\n\n` +
                'No responses expected (all were notifications without id field).';
            return;
        }
        
        // Wait a bit for responses to come in, then collect them
        // Use a message handler to capture responses
        const responses = [];
        const requestIds = new Set(requests.map(r => r.id));
        
        const handler = (msg) => {
            if (msg.id && requestIds.has(msg.id)) {
                responses.push(msg);
                requestIds.delete(msg.id);
                
                // Update display as responses come in
                const duration = Date.now() - startTime;
                let display = `⏳ Receiving responses... (${duration}ms)\n`;
                display += `Requests: ${requests.length}, Notifications: ${notifications.length}\n`;
                display += `Received: ${responses.length}/${requests.length}\n\n`;
                display += `Responses so far:\n${JSON.stringify(responses, null, 2)}`;
                responseBox.textContent = display;
                
                // All responses received
                if (requestIds.size === 0) {
                    client.off('message', handler);
                    display = `✅ Batch complete! (${duration}ms)\n`;
                    display += `Requests: ${requests.length}, Notifications: ${notifications.length}\n\n`;
                    display += `Responses:\n${JSON.stringify(responses, null, 2)}`;
                    responseBox.textContent = display;
                }
            }
        };
        
        client.on('message', handler);
        
        // Set a timeout in case not all responses come back
        setTimeout(() => {
            if (requestIds.size > 0) {
                client.off('message', handler);
                const duration = Date.now() - startTime;
                let display = `⚠️ Batch timeout! (${duration}ms)\n`;
                display += `Requests: ${requests.length}, Notifications: ${notifications.length}\n`;
                display += `Received: ${responses.length}/${requests.length}\n`;
                display += `Missing responses for IDs: ${Array.from(requestIds).join(', ')}\n\n`;
                display += `Responses received:\n${JSON.stringify(responses, null, 2)}`;
                responseBox.textContent = display;
            }
        }, 10000); // 10 second timeout
        
    } catch (error) {
        responseBox.textContent = 'Error: ' + error.message;
    }
}

// Console Functions
function addConsoleEntry(message, type = 'info') {
    const consoleLog = document.getElementById('consoleLog');
    if (!consoleLog) return;
    
    const entry = document.createElement('div');
    entry.className = 'console-entry';
    
    const typeClass = 'console-' + type;
    entry.innerHTML = `<span class="${typeClass}">${escapeHtml(message)}</span>`;
    
    consoleLog.appendChild(entry);
    
    // Auto-scroll if enabled
    const autoscroll = document.getElementById('autoscroll');
    if (autoscroll && autoscroll.checked) {
        consoleLog.scrollTop = consoleLog.scrollHeight;
    }
}

function clearConsole() {
    const consoleLog = document.getElementById('consoleLog');
    consoleLog.innerHTML = '';
    addConsoleEntry('Console cleared', 'info');
}

// Utility Functions
function escapeHtml(text) {
    const div = document.createElement('div');
    div.textContent = text;
    return div.innerHTML;
}

// Server Logs Functions
async function subscribeToServerLogs() {
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (serverLogsSubscribed) {
        alert('Already subscribed to server logs');
        return;
    }

    try {
        await client.subscribe('server.logs', (data) => {
            addServerLog(data);
        });
        
        serverLogsSubscribed = true;
        updateServerLogsStatus();
        alert('Subscribed to server logs! Server will push log messages here.');
    } catch (error) {
        alert('Failed to subscribe to server logs: ' + JSON.stringify(error));
    }
}

async function unsubscribeFromServerLogs() {
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (!serverLogsSubscribed) {
        alert('Not subscribed to server logs');
        return;
    }

    try {
        await client.unsubscribe('server.logs');
        serverLogsSubscribed = false;
        updateServerLogsStatus();
        alert('Unsubscribed from server logs');
    } catch (error) {
        alert('Failed to unsubscribe: ' + JSON.stringify(error));
    }
}

function addServerLog(data) {
    const container = document.getElementById('serverLogsContainer');
    if (!container) return;

    const time = new Date().toLocaleTimeString();
    const level = data.level || 'info';
    const message = data.message || JSON.stringify(data);
    
    const entry = document.createElement('div');
    entry.className = 'server-log-entry';
    
    const timeSpan = document.createElement('span');
    timeSpan.className = 'server-log-time';
    timeSpan.textContent = `[${time}]`;
    
    const levelSpan = document.createElement('span');
    levelSpan.className = `server-log-level ${level}`;
    levelSpan.textContent = level.toUpperCase();
    
    const messageSpan = document.createElement('span');
    messageSpan.className = 'server-log-message';
    messageSpan.textContent = message;
    
    entry.appendChild(timeSpan);
    entry.appendChild(levelSpan);
    entry.appendChild(messageSpan);
    
    container.appendChild(entry);
    
    // Update count
    serverLogsCount++;
    document.getElementById('serverLogsCount').textContent = serverLogsCount;
    
    // Auto-scroll if enabled
    const autoscroll = document.getElementById('serverLogsAutoscroll');
    if (autoscroll && autoscroll.checked) {
        container.scrollTop = container.scrollHeight;
    }
}

function clearServerLogs() {
    document.getElementById('serverLogsContainer').innerHTML = '';
    serverLogsCount = 0;
    document.getElementById('serverLogsCount').textContent = '0';
}

function updateServerLogsStatus() {
    const statusEl = document.getElementById('serverLogsStatus');
    if (serverLogsSubscribed) {
        statusEl.textContent = '🟢 Subscribed';
        statusEl.style.color = 'var(--success-color)';
    } else {
        statusEl.textContent = '⚫ Not subscribed';
        statusEl.style.color = 'var(--text-muted)';
    }
}

// Persistent Subscription Functions
const persistentSubscriptions = new Map(); // Map of subscription_id -> {topic, messages}

async function subscribePersistent() {
    const topic = document.getElementById('persistentTopic').value.trim();
    const subscriptionId = document.getElementById('subscriptionId').value.trim();
    
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (!topic || !subscriptionId) {
        alert('Please enter both topic and subscription ID');
        return;
    }

    try {
        // Subscribe to receive persistent messages as notifications
        const handler = (data) => {
            // Data contains: sequence_id, topic, data
            // Add subscription_id so we can acknowledge it later
            addPersistentMessage(data, subscriptionId);
        };
        
        // Register the handler for this topic
        await client.subscribe(topic, handler);
        
        // Now request persistent subscription
        const result = await client.request('rpc.subscribe_persistent', {
            topic: topic,
            subscription_id: subscriptionId
        });
        
        persistentSubscriptions.set(subscriptionId, {
            topic: topic,
            handler: handler,
            messages: []
        });
        
        updatePersistentSubscriptionsList();
        
        const msg = `Subscribed to persistent topic: ${topic}\n` +
                    `Subscription ID: ${subscriptionId}\n` +
                    `Resumed from sequence: ${result.resumed_from_seq || 0}\n` +
                    `Undelivered messages: ${result.undelivered_count || 0}`;
        alert(msg);
    } catch (error) {
        alert('Failed to subscribe: ' + JSON.stringify(error));
    }
}

async function unsubscribePersistent() {
    const subscriptionId = document.getElementById('subscriptionId').value.trim();
    
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    if (!subscriptionId) {
        alert('Please enter subscription ID');
        return;
    }

    const sub = persistentSubscriptions.get(subscriptionId);
    if (!sub) {
        alert('Subscription not found');
        return;
    }

    try {
        // Unsubscribe from the persistent subscription
        await client.request('rpc.unsubscribe_persistent', {
            subscription_id: subscriptionId
        });
        
        // Also unsubscribe from the regular topic subscription
        await client.unsubscribe(sub.topic);
        
        persistentSubscriptions.delete(subscriptionId);
        updatePersistentSubscriptionsList();
        alert(`Unsubscribed from: ${subscriptionId}`);
    } catch (error) {
        alert('Failed to unsubscribe: ' + JSON.stringify(error));
    }
}

async function acknowledgePersistentMessage(subscriptionId, sequenceId) {
    if (!client.isConnected()) {
        alert('Not connected to server');
        return;
    }

    try {
        await client.request('rpc.ack_persistent', {
            subscription_id: subscriptionId,
            sequence_id: sequenceId
        });
        
        // Remove from UI
        const messageEl = document.querySelector(`[data-seq="${sequenceId}"][data-sub="${subscriptionId}"]`);
        if (messageEl) {
            messageEl.remove();
        }
        
        console.log(`Acknowledged message - Sub: ${subscriptionId}, Seq: ${sequenceId}`);
    } catch (error) {
        alert('Failed to acknowledge: ' + JSON.stringify(error));
    }
}

function updatePersistentSubscriptionsList() {
    const container = document.getElementById('activePersistentSubscriptions');
    
    if (persistentSubscriptions.size === 0) {
        container.innerHTML = '<em class="empty-state">No active persistent subscriptions</em>';
        return;
    }
    
    container.innerHTML = '';
    persistentSubscriptions.forEach((sub, subId) => {
        const item = document.createElement('div');
        item.className = 'subscription-item';
        item.innerHTML = `
            <span class="topic-name">${sub.topic}</span>
            <span class="sub-id">(ID: ${subId})</span>
            <button class="btn-remove" onclick="document.getElementById('subscriptionId').value='${subId}'; unsubscribePersistent()">✖</button>
        `;
        container.appendChild(item);
    });
}

function addPersistentMessage(notificationData, subscriptionId) {
    const container = document.getElementById('persistentMessages');
    if (!container) return;

    const time = new Date().toLocaleTimeString();
    // Data structure from server: { sequence_id, topic, data }
    const seq = notificationData.sequence_id || notificationData.sequence_number || 'unknown';
    const actualTopic = notificationData.topic || 'unknown';
    const message = notificationData.data || notificationData;
    
    const messageDiv = document.createElement('div');
    messageDiv.className = 'subscription-message';
    messageDiv.setAttribute('data-seq', seq);
    messageDiv.setAttribute('data-sub', subscriptionId);
    
    const headerDiv = document.createElement('div');
    headerDiv.className = 'message-time';
    headerDiv.innerHTML = `[${time}] <strong>Seq: ${seq}</strong> | Topic: ${actualTopic}`;
    
    const dataDiv = document.createElement('pre');
    dataDiv.className = 'message-data';
    dataDiv.textContent = JSON.stringify(message, null, 2);
    
    const ackButton = document.createElement('button');
    ackButton.className = 'btn btn-small btn-success';
    ackButton.textContent = '✓ Ack';
    ackButton.onclick = () => acknowledgePersistentMessage(subscriptionId, seq);
    
    messageDiv.appendChild(headerDiv);
    messageDiv.appendChild(dataDiv);
    messageDiv.appendChild(ackButton);
    
    container.appendChild(messageDiv);
    container.scrollTop = container.scrollHeight;
}

function clearPersistentMessages() {
    document.getElementById('persistentMessages').innerHTML = '';
}

// Keyboard shortcuts
document.addEventListener('keydown', (event) => {
    // Ctrl/Cmd + Enter to send request
    if ((event.ctrlKey || event.metaKey) && event.key === 'Enter') {
        const activePanel = document.querySelector('.panel.active');
        if (!activePanel) return;
        
        switch (activePanel.id) {
            case 'request':
                sendRequest();
                break;
            case 'notify':
                sendNotification();
                break;
            case 'batch':
                sendBatch();
                break;
        }
    }
});

// Add tooltip for keyboard shortcuts
window.addEventListener('load', () => {
    const panels = ['request', 'notify', 'batch'];
    panels.forEach(panelId => {
        const panel = document.getElementById(panelId);
        if (panel) {
            const button = panel.querySelector('.btn-primary');
            if (button) {
                button.title = 'Click or press Ctrl+Enter';
            }
        }
    });
});

