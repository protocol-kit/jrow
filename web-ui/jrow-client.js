/**
 * JROW Client - JSON-RPC 2.0 over WebSocket
 * 
 * A lightweight JavaScript client for communicating with JROW servers.
 * Supports requests, notifications, subscriptions, and batch operations.
 */

class JrowClient {
    constructor() {
        this.ws = null;
        this.requestId = 1;
        this.pendingRequests = new Map();
        this.subscriptions = new Map();
        this.messageHandlers = [];
        this.connectionHandlers = {
            onOpen: [],
            onClose: [],
            onError: []
        };
    }

    /**
     * Connect to a JROW server
     * @param {string} url - WebSocket URL (ws:// or wss://)
     * @returns {Promise<void>}
     */
    connect(url) {
        return new Promise((resolve, reject) => {
            if (this.ws && this.ws.readyState === WebSocket.OPEN) {
                reject(new Error('Already connected'));
                return;
            }

            try {
                this.ws = new WebSocket(url);
            } catch (error) {
                reject(error);
                return;
            }

            this.ws.onopen = () => {
                this.log('Connected to ' + url, 'success');
                this.connectionHandlers.onOpen.forEach(handler => handler());
                resolve();
            };

            this.ws.onerror = (error) => {
                this.log('WebSocket error: ' + error, 'error');
                this.connectionHandlers.onError.forEach(handler => handler(error));
                reject(error);
            };

            this.ws.onclose = (event) => {
                this.log(`Disconnected (code: ${event.code})`, 'warning');
                this.connectionHandlers.onClose.forEach(handler => handler(event));
                
                // Reject all pending requests
                this.pendingRequests.forEach((pending, id) => {
                    pending.reject(new Error('Connection closed'));
                });
                this.pendingRequests.clear();
            };

            this.ws.onmessage = (event) => {
                this.handleMessage(event.data);
            };
        });
    }

    /**
     * Disconnect from the server
     */
    disconnect() {
        if (this.ws) {
            this.ws.close();
            this.ws = null;
        }
    }

    /**
     * Check if connected
     * @returns {boolean}
     */
    isConnected() {
        return this.ws && this.ws.readyState === WebSocket.OPEN;
    }

    /**
     * Handle incoming WebSocket message
     * @private
     */
    handleMessage(data) {
        try {
            const message = JSON.parse(data);
            this.log('← ' + data, 'receive');

            // Notify all message handlers
            this.messageHandlers.forEach(handler => handler(message, 'receive'));

            // Handle batch response
            if (Array.isArray(message)) {
                message.forEach(msg => this.handleSingleMessage(msg));
                return;
            }

            this.handleSingleMessage(message);
        } catch (error) {
            this.log('Parse error: ' + error.message, 'error');
        }
    }

    /**
     * Handle a single JSON-RPC message
     * @private
     */
    handleSingleMessage(message) {
        // Call custom message handlers first
        this.messageHandlers.forEach(handler => {
            try {
                handler(message);
            } catch (error) {
                console.error('Message handler error:', error);
            }
        });
        
        // Handle response (has id and result/error)
        if (message.id !== undefined && (message.result !== undefined || message.error)) {
            const pending = this.pendingRequests.get(message.id);
            if (pending) {
                if (message.error) {
                    pending.reject(message.error);
                } else {
                    pending.resolve(message.result);
                }
                this.pendingRequests.delete(message.id);
            }
        }
        // Handle notification (has method but no id)
        else if (message.method && message.id === undefined) {
            // Check for topic-specific subscription handlers
            const handler = this.subscriptions.get(message.method);
            if (handler) {
                handler(message.params);
            }

            // Also check for rpc.notification format
            if (message.method === 'rpc.notification' && message.params) {
                const topic = message.params.topic;
                const data = message.params.data;
                const topicHandler = this.subscriptions.get(topic);
                if (topicHandler) {
                    topicHandler(data);
                }
            }
        }
    }

    /**
     * Send a JSON-RPC request and wait for response
     * @param {string} method - Method name
     * @param {*} params - Parameters (will be JSON serialized)
     * @param {number} timeout - Request timeout in ms (default: 30000)
     * @returns {Promise<*>} - Response result
     */
    request(method, params = null, timeout = 30000) {
        return new Promise((resolve, reject) => {
            if (!this.isConnected()) {
                reject(new Error('Not connected'));
                return;
            }

            const id = this.requestId++;
            const request = {
                jsonrpc: "2.0",
                method,
                id
            };

            if (params !== null && params !== undefined) {
                request.params = params;
            }

            // Store pending request
            this.pendingRequests.set(id, { resolve, reject });

            // Set timeout
            const timeoutId = setTimeout(() => {
                if (this.pendingRequests.has(id)) {
                    this.pendingRequests.delete(id);
                    reject(new Error(`Request timeout after ${timeout}ms`));
                }
            }, timeout);

            // Clear timeout when resolved/rejected
            const originalResolve = resolve;
            const originalReject = reject;
            
            this.pendingRequests.set(id, {
                resolve: (result) => {
                    clearTimeout(timeoutId);
                    originalResolve(result);
                },
                reject: (error) => {
                    clearTimeout(timeoutId);
                    originalReject(error);
                }
            });

            // Send request
            this.send(request);
        });
    }

    /**
     * Send a notification (no response expected)
     * @param {string} method - Method name
     * @param {*} params - Parameters
     */
    notify(method, params = null) {
        if (!this.isConnected()) {
            throw new Error('Not connected');
        }

        const notification = {
            jsonrpc: "2.0",
            method
        };

        if (params !== null && params !== undefined) {
            notification.params = params;
        }

        this.send(notification);
    }

    /**
     * Subscribe to a topic
     * @param {string} topic - Topic name (supports patterns like "events.*")
     * @param {Function} handler - Handler function for received messages
     * @returns {Promise<*>} - Subscription result
     */
    async subscribe(topic, handler) {
        // Store the handler
        this.subscriptions.set(topic, handler);

        try {
            // Send subscription request to server
            const result = await this.request('rpc.subscribe', { topic });
            this.log(`Subscribed to topic: ${topic}`, 'success');
            return result;
        } catch (error) {
            // Remove handler if subscription failed
            this.subscriptions.delete(topic);
            throw error;
        }
    }

    /**
     * Unsubscribe from a topic
     * @param {string} topic - Topic name
     * @returns {Promise<*>} - Unsubscription result
     */
    async unsubscribe(topic) {
        try {
            const result = await this.request('rpc.unsubscribe', { topic });
            this.subscriptions.delete(topic);
            this.log(`Unsubscribed from topic: ${topic}`, 'success');
            return result;
        } catch (error) {
            // Still remove handler even if server request failed
            this.subscriptions.delete(topic);
            throw error;
        }
    }

    /**
     * Get list of active subscriptions
     * @returns {string[]} - Array of topic names
     */
    getSubscriptions() {
        return Array.from(this.subscriptions.keys());
    }

    /**
     * Send a batch of requests
     * @param {Array} requests - Array of request/notification objects
     * @returns {Promise<void>}
     */
    sendBatch(requests) {
        if (!this.isConnected()) {
            throw new Error('Not connected');
        }

        if (!Array.isArray(requests) || requests.length === 0) {
            throw new Error('Batch must be a non-empty array');
        }

        this.send(requests);
    }

    /**
     * Send raw JSON-RPC message
     * @private
     */
    send(message) {
        const json = JSON.stringify(message);
        this.log('→ ' + json, 'send');
        this.ws.send(json);

        // Notify message handlers
        this.messageHandlers.forEach(handler => handler(message, 'send'));
    }

    /**
     * Register a connection event handler
     * @param {string} event - Event name ('open', 'close', 'error', 'message')
     * @param {Function} handler - Handler function
     */
    on(event, handler) {
        if (event === 'open') {
            this.connectionHandlers.onOpen.push(handler);
        } else if (event === 'close') {
            this.connectionHandlers.onClose.push(handler);
        } else if (event === 'error') {
            this.connectionHandlers.onError.push(handler);
        } else if (event === 'message') {
            this.messageHandlers.push(handler);
        }
    }

    /**
     * Remove an event handler
     * @param {string} event - Event name ('open', 'close', 'error', 'message')
     * @param {Function} handler - Handler function to remove
     */
    off(event, handler) {
        if (event === 'open') {
            const idx = this.connectionHandlers.onOpen.indexOf(handler);
            if (idx > -1) this.connectionHandlers.onOpen.splice(idx, 1);
        } else if (event === 'close') {
            const idx = this.connectionHandlers.onClose.indexOf(handler);
            if (idx > -1) this.connectionHandlers.onClose.splice(idx, 1);
        } else if (event === 'error') {
            const idx = this.connectionHandlers.onError.indexOf(handler);
            if (idx > -1) this.connectionHandlers.onError.splice(idx, 1);
        } else if (event === 'message') {
            const idx = this.messageHandlers.indexOf(handler);
            if (idx > -1) this.messageHandlers.splice(idx, 1);
        }
    }

    /**
     * Log a message (can be overridden)
     * @param {string} message - Message to log
     * @param {string} type - Message type ('send', 'receive', 'success', 'error', 'warning')
     */
    log(message, type = 'info') {
        // Override this method to customize logging
        console.log(`[JROW] ${type.toUpperCase()}: ${message}`);
    }
}

// Make JrowClient available globally
if (typeof window !== 'undefined') {
    window.JrowClient = JrowClient;
}

// Export for module systems
if (typeof module !== 'undefined' && module.exports) {
    module.exports = JrowClient;
}

