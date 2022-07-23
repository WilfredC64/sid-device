// Copyright (C) 2022 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

export default class {
    static handleKeyDown = (event) => {
        if (event.ctrlKey) {
            switch (event.code) {
                case 'F5':
                case 'KeyF':
                case 'KeyG':
                case 'KeyP':
                case 'KeyR':
                case 'KeyS':
                case 'KeyU': {
                    event.preventDefault();
                }
            }
        }
    }

    static disableDefaultKeys = () => {
        if (window.location.hostname === 'localhost') {
            return
        }

        addEventListener('keydown', this.handleKeyDown, false);
    }

    static disableContextMenu = () => {
        if (window.location.hostname === 'localhost') {
            return
        }

        addEventListener('contextmenu', event => event.preventDefault(), false);
        addEventListener('selectstart', event => event.preventDefault(), false);
    }
}
