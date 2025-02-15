<!--
  Copyright (C) 2025 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div id="title-bar" @mousedown="startDragging" @mouseleave="stopDragging"></div>
    <div ref="closeButton" id="close-button" :class="'disable-close-button'" @mousemove="keepButtonVisible" @mouseleave="enableCloseButton" @click="closeWindow">&#10005;</div>
</template>

<script>

import { onBeforeUnmount, ref } from 'vue';
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
            window.removeEventListener('blur', closeWindow);
            appWindow.startDragging();
        };

        const stopDragging = () => {
            window.removeEventListener('blur', closeWindow);
            window.addEventListener('blur', closeWindow);
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

        const handleSpecialKeys = (event) => {
            if (event.key === 'Escape') {
                if (document.activeElement === document.body) {
                    event.preventDefault();
                    appWindow.close();
                } else {
                    document.body.setAttribute('tabindex', '-1');
                    document.body.focus();
                }
                return;
            }

            if (event.key === 'Tab') {
                const focusableElements = document.querySelectorAll('[tabindex]:not([tabindex="-1"])');
                if (focusableElements.length === 0) {
                    event.preventDefault();
                    return;
                }

                const firstElement = focusableElements[0];
                const lastElement = focusableElements[focusableElements.length - 1];
                const isShiftPressed = event.shiftKey;
                const activeElement = document.activeElement;

                if (activeElement === lastElement && !isShiftPressed) {
                    event.preventDefault();
                    firstElement.focus();
                } else if ((activeElement === firstElement || activeElement === document.body) && isShiftPressed) {
                    event.preventDefault();
                    lastElement.focus();
                }
            }
        };

        onBeforeUnmount(() => {
            document.removeEventListener('mousemove', enableCloseButton);
            document.removeEventListener('mouseout', hideCloseButton);
            window.removeEventListener('blur', closeWindow);
            window.removeEventListener('keydown', handleSpecialKeys);
        });

        document.addEventListener('mousemove', enableCloseButton);
        document.addEventListener('mouseout', hideCloseButton);
        window.addEventListener('blur', closeWindow);
        window.addEventListener('keydown', handleSpecialKeys);

        return {
            closeWindow,
            enableCloseButton,
            keepButtonVisible,
            showCloseButton,
            hideCloseButton,
            startDragging,
            stopDragging,
            handleSpecialKeys,
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