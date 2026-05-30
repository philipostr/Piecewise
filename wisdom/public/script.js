import * as State from './wisdom-js/state.js';

function gen_sub_id() {
    return crypto.randomUUID().split("-")[0];
}

load_Game();

