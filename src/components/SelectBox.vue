<!--
  Copyright (C) 2022 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div class="select-box-wrapper">
        <div tabindex="0" ref="selectBox" class="select-box" @click="toggleOptions" @keydown="handleKeyDown">
            <span class="selected-item">{{ options[selectedIndex] }}</span>
            <arrow-down class="drop-down-arrow" alt="arrow down" @drag="false"></arrow-down>
        </div>
        <div ref="options" class="options-wrapper" :class="{'options-expanded': optionsExpanded}">
            <div tabindex="0" ref="optionElements" class="options" @keydown="handleKeyDown" @blur="closeOptions">
                <div
                    class="option"
                    :class="{'options-selected': selectedIndex === index}"
                    @click="selectOption($event, index)"
                    v-for="(option, index) in options" :key="index">
                    {{ option }}
                </div>
            </div>
        </div>
    </div>
</template>

<script>

import ArrowDown from '../assets/arrow-down.svg';
import { nextTick, onMounted, onUnmounted, ref, watch } from 'vue';

export default {
    name: 'select-box',
    props: {
        options: {
            type: Array,
            default: []
        },
        selectedIndex: {
            type: Number,
            default: 0
        }
    },
    emits: ["change"],
    setup(props, ctx) {
        let optionsExpanded = ref(false);
        let nextOnclickIgnored = ref(false);
        const optionElements = ref(null);
        const selectBox = ref(null);

        let items = [];

        watch(() => props.options, () => {
            nextTick(() => {
                if (optionElements) {
                    items = optionElements.value.querySelectorAll('div.option');
                }
            })
        });

        watch(() => props.selectedIndex, () => {
            showCurrentSelection(false);
        });

        const showCurrentSelection = (centerSelected) => {
            nextTick(() => {
                optionElements.value.focus();

                if (items.length > 0 && props.selectedIndex >= 0 && props.selectedIndex < items.length) {
                    const element = items[props.selectedIndex];
                    const optionsPos = optionElements.value.getBoundingClientRect();
                    const elementPos = element.getBoundingClientRect();

                    if (centerSelected) {
                        const newTopY = elementPos.height * props.selectedIndex;
                        const middlePos = Math.trunc((optionsPos.height - elementPos.height) / elementPos.height / 2) * elementPos.height;
                        if (newTopY < middlePos) {
                            optionElements.value.scrollTop = 0;
                        } else {
                            optionElements.value.scrollTop = newTopY - middlePos;
                        }
                    } else if (elementPos.y + elementPos.height >= optionsPos.y + optionsPos.height) {
                        element.scrollIntoView(false);
                    } else if (elementPos.y + elementPos.height <= optionsPos.y) {
                        element.scrollIntoView(true);
                    }
                }
            });
        }

        const toggleOptions = () => {
            if (!nextOnclickIgnored.value) {
                optionsExpanded.value = !optionsExpanded.value;
                if (optionsExpanded.value) {
                    showCurrentSelection(true);
                }
            }
            nextOnclickIgnored.value = false;
        }

        const closeOptions = (event) => {
            if (optionsExpanded.value) {
                optionsExpanded.value = false;
                selectBox.value.focus();

                if (event?.relatedTarget?.className === 'select-box') {
                    nextOnclickIgnored.value = true;
                }
            }
        }

        const enableNextOnClick = () => {
            nextOnclickIgnored.value = false
        }

        const selectOption = (event, index) => {
            closeOptions();
            ctx.emit('change', index);
        }

        const getPageUpStep = () => {
            const element = items[props.selectedIndex];
            const optionsPos = optionElements.value.getBoundingClientRect();
            const elementPos = element.getBoundingClientRect();
            return Math.trunc(optionsPos.height / elementPos.height) - 1;
        }

        const handleKeyDown = (event) => {
            switch (event.code) {
                case 'Tab': {
                    optionsExpanded.value = false;
                    enableNextOnClick();
                    break;
                }
                case 'Home': {
                    const newIndex = 0;
                    if (props.selectedIndex !== newIndex && props.options.length > 0) {
                        ctx.emit('change', newIndex);
                    }
                    event.preventDefault();
                    break;
                }
                case 'End': {
                    const newIndex = props.options.length - 1;
                    if (props.selectedIndex !== newIndex && props.options.length > 0) {
                        ctx.emit('change', newIndex);
                    }
                    event.preventDefault();
                    break;
                }
                case 'ArrowUp': {
                    if (props.selectedIndex - 1 >= 0) {
                        ctx.emit('change', props.selectedIndex - 1);
                    }
                    event.preventDefault();
                    break;
                }
                case 'ArrowDown': {
                    if (props.selectedIndex + 1 < props.options.length) {
                        ctx.emit('change', props.selectedIndex + 1);
                    }
                    event.preventDefault();
                    break;
                }
                case 'Escape':
                case 'Enter': {
                    if (optionsExpanded.value) {
                        optionsExpanded.value = false;
                        selectBox.value.focus();
                        event.preventDefault();
                        event.stopPropagation();
                    }
                    break;
                }
                case 'Space': {
                    if (props.options.length > 0 && !optionsExpanded.value) {
                        optionsExpanded.value = true;
                        showCurrentSelection(true);
                    }
                    event.preventDefault();
                    break;
                }
                case 'PageUp': {
                    if (props.options.length > 0) {
                        let newIndex = props.selectedIndex - getPageUpStep();
                        if (newIndex < 0) {
                            newIndex = 0;
                        }
                        if (props.selectedIndex !== newIndex) {
                            ctx.emit('change', newIndex)
                            showCurrentSelection(false);
                        }
                    }
                    event.preventDefault();
                    break;
                }
                case 'PageDown': {
                    if (props.options.length > 0) {
                        let newIndex = props.selectedIndex + getPageUpStep();
                        if (newIndex >= props.options.length) {
                            newIndex = props.options.length - 1;
                        }
                        if (props.selectedIndex !== newIndex) {
                            ctx.emit('change', newIndex)
                            showCurrentSelection(false);
                        }
                    }
                    event.preventDefault();
                    break;
                }
            }
        };

        onMounted(() => {
            addEventListener('click', enableNextOnClick, false);
        });

        onUnmounted(() => {
            removeEventListener('click', enableNextOnClick);
        });

        return {
            optionElements,
            optionsExpanded,
            selectBox,
            nextOnclickIgnored,
            closeOptions,
            handleKeyDown,
            selectOption,
            toggleOptions
        }
    },
    components: {
        ArrowDown
    }
};

</script>

<style scoped>

.select-box-wrapper {
    margin-bottom: 8px;
}

.select-box {
    border: 1px solid rgba(75, 75, 95, .60);
    border-radius: 2px;
    text-shadow: none;

    padding: 6px 4px;
    margin-bottom: 4px;

    font-size: 15px;
    height: 17px;
    color: #010105;
    background-color: rgba(235, 235, 255, .60);
    box-shadow: 0 0 0 2px rgba(50, 50, 80, .4);

    cursor: pointer;

    display: flex;
    justify-content: space-between;
}

.drop-down-arrow {
    width: 18px;
    height: 18px;
    fill: #26262A;
}

.options-wrapper {
    position: relative;
    width: 100%;
    visibility: hidden;
}

.options-expanded {
    visibility: visible;
}

.options-selected {
    background-color: #606074 !important;
    color: #d4d0e0 !important;;
}

.options {
    background-color: #B0B0C4;
    color: #010105;
    text-shadow: none;

    font-size: 15px;

    position: absolute;
    z-index: 100;

    width: 100%;

    overflow-y: auto;
    max-height: 202px;

    box-shadow: 0 0 0 2px rgba(50, 50, 80, .4);

    border: 1px solid rgba(75, 75, 95, .60);
    border-radius: 2px;

    box-sizing: border-box;
}

.option {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;

    padding: 4px;
}

.option:hover {
    background-color: rgba(105, 105, 125, .60);
    color: #d4d0e0;
}

.selected-item {
    overflow: hidden;
    white-space: nowrap;
    text-overflow: ellipsis;
}

</style>
