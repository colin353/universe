class GlobalStore {
    constructor() {
        this.state = {};
        this.listeners = {};
    }

    addWatcher(key, listenerId, updateFn) {
        if(!this.listeners[listenerId]) {
            this.listeners[listenerId] = {};
        }
        this.listeners[listenerId][key] = updateFn;
        updateFn(this.state[key]);
    }

    removeWatcher(listenerId) {
        this.listeners[listenerId] = {};
    }

    setState(key, value) {
        this.state[key] = value;
        for(const listener of Object.values(this.listeners)) {
            if (listener[key]) {
                listener[key](value);
            }
        }
    }
}

export default class Store {
    constructor() {
        this.listenerId = "";
        for (let x=0;x<16;x++) {
            this.listenerId += (Math.random() * 16 | 0).toString(16);
        }
    }

    getState(key) {
        return globalThis.__globalStore.state[key];
    }

    setState(key, value) {
        globalThis.__globalStore.setState(key, value);
    }

    addWatcher(key, updateFn) {
        globalThis.__globalStore.addWatcher(key, this.listenerId, updateFn);
    }
}

if(!globalThis.__globalStore) {
    globalThis.__globalStore = new GlobalStore();
}
