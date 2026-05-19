export function createInt(initialValue, data) {
    var state = initialValue;
    var stateId = crypto.randomUUID();

    var snapshot = () => {
        return state;
    }

    var subscribe = (callback) => {
        let f = (e) => {
            callback(e.detail.newValue);
        };
        document.addEventListener(stateId, f);

        return () => {
            document.removeEventListener(stateId, f);
        };
    }

    var mutate = (mutation) => {
        let newState = mutation(state);
        if (Object.hasOwn(data, "minimum") && newState < data.minimum) {
            return;
        }
        if (Object.hasOwn(data, "maximum") && newState > data.maximum) {
            return;
        }
        
        let oldState = state;
        state = newState;
        
        document.dispatchEvent(new CustomEvent(stateId, {
            detail: {
                newValue: state,
                oldValue: oldState
            }
        }));
    }
    
    return [snapshot, subscribe, mutate];
}

export function snapshot(state) {
    return state[0]();
}

export function subscribe(state, callback) {
    return state[1](callback);
}

export function mutate(state, mutation) {
    state[2](mutation);
}
