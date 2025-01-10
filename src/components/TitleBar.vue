<!--
  Copyright (C) 2025 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div id="title-bar" data-tauri-drag-region @mousedown="startDragging"></div>
    <div ref="closeButton" id="close-button" :class="'disable-close-button'" @mousemove="keepButtonVisible" @mouseleave="enableCloseButton" @click="closeWindow">&#10005;</div>
</template>

<script>

import { onBeforeUnmount, ref, watch } from 'vue';
import { getCurrentWindow  } from "@tauri-apps/api/window";

export default {
    name: 'TitleBar',
    setup() {
        const hideButtonTimer = ref(null);
        const closeButton = ref();
        const appWindow = getCurrentWindow();

        const closeWindow = () => {
            if (closeButton.value) {
                closeButton.value.classList.add('disable-close-button');
            }
            appWindow.close();
        };

        const startDragging = () => {
            appWindow.startDragging();
        };

        const enableCloseButton = () => {
            showCloseButton();

            hideButtonTimer.value = setTimeout(() => {
                hideCloseButton();
                hideButtonTimer.value = null;
            }, 1000);
        };

        const keepButtonVisible = (event) => {
            event.preventDefault();
            event.stopPropagation();
            showCloseButton();
        }

        const showCloseButton = () => {
            if (closeButton.value) {
                closeButton.value.classList.remove('disable-close-button');

                if (hideButtonTimer.value) {
                    clearTimeout(hideButtonTimer.value);
                }
            }
        };

        const hideCloseButton = () => {
            if (closeButton.value) {
                closeButton.value.classList.add('disable-close-button');

            }
        };

        onBeforeUnmount(() => {
            document.removeEventListener('mousemove', enableCloseButton);
            document.removeEventListener('mouseleave', hideCloseButton);
        });

        document.addEventListener('mousemove', enableCloseButton);
        document.addEventListener('mouseleave', hideCloseButton);

        return {
            closeWindow,
            enableCloseButton,
            keepButtonVisible,
            showCloseButton,
            hideCloseButton,
            startDragging,
            closeButton
        };
    }
};

</script>

<style scoped>

#title-bar {
    position: absolute;
    top: 0;
    width: 100%;
    height: 30px;
    color: white;
    display: flex;
    align-items: center;
}

#close-button {
    padding: 2px;
    position: absolute;
    right: 4px;
    top: 3px;
    width: 18px;
    height: 18px;
    color: #7F7F92;
    font-size: 1rem;
    opacity: 1;
    font-weight: bold;
    transition: opacity 300ms ease-in-out;
}

#close-button:hover {
    color: #d4d0e0;
    cursor: pointer;
    opacity: 1;
}

.disable-close-button {
    opacity: 0 !important;
}

</style>