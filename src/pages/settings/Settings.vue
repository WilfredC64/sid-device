<!--
  Copyright (C) 2022 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <SettingsDialog ref="settingsRef"/>
</template>

<script>

import SettingsDialog from '@/components/SettingsDialog.vue'
import { listen } from '@tauri-apps/api/event';
import { ref } from 'vue'
import tauri from '../../utils/tauri';

export default {
    name: 'Settings',
    setup() {
        const settingsRef = ref(null);

        tauri.disableContextMenu();
        tauri.disableDefaultKeys();

        const installListeners = async () => {
            await listen('update-settings', async (config) => {
                settingsRef.value?.setConfig(config.payload);
            });
        }

        installListeners();

        return {settingsRef}
    },
    components: {
        SettingsDialog
    }
}

</script>

<style>

#settings {
    font-family: Arial, sans-serif;
    -webkit-font-smoothing: antialiased;
    -moz-osx-font-smoothing: grayscale;

    color: #9F9FA4;
    height: 100%;

    text-shadow: 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000, 0 0 2px #000000;
}

html {
    background-color: black;
    height: 100%;
}

body {
    height: 100%;
    user-select: none;
    overflow: hidden;
}

* {
    margin: 0;
    padding: 0;
}

::-webkit-scrollbar {
    width: 12px;
}

::-webkit-scrollbar-thumb:vertical {
    background: radial-gradient(circle, #585872, #292A46);
    box-shadow: inset 1px 0 #0E0B32, inset 0 0 #0E0B32, inset 2px 1px 2px hsla(0, 0%, 100%, .15), inset -2px -1px 2px rgba(0, 0, 0, .15);
}

::-webkit-scrollbar-thumb:hover {
    background: radial-gradient(circle, #787892, #494A66);
}

::-webkit-scrollbar-track:vertical {
    background: #808094;
    border-left: 1px solid #0a133d;
}

::-webkit-scrollbar-track:horizontal {
    background: #808094;
    border-top: 1px solid #0a133d;
}

::-webkit-scrollbar-corner {
    background: #808094;
}

</style>
