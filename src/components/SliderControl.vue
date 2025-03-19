<!--
  Copyright (C) 2022 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div ref="progressBorder" class="progress-border" @mousedown="setPosAndMove" @touchstart="setPosAndMove" @mouseenter="colorPosition" @mousemove="colorPosition">
        <div ref="progressWrapper" @keydown="handleKeyDown" class="progress-wrapper">
            <div ref="sliderValue" class="slider-value">100</div>
            <div class="progress-bar" tabindex="0">
                <div ref="indicatorWrapper" class="indicator-wrapper">
                  <indicator ref="indicator" class="indicator" alt="value indicator" @drag="false" @dblclick="setDefaultValue" @mousedown="initMoveIndicator" @touchstart="initMoveIndicator"></indicator>
                </div>
                <div ref="progress" class="progress">
                    <div ref="sliderBarLeft" class="slider-bar-left"></div>
                    <div ref="sliderBar" class="slider-bar"></div>
                    <div ref="sliderBarRight" class="slider-bar-right"></div>
                </div>
            </div>
        </div>
    </div>
</template>

<script>

import Indicator from '../assets/indicator.svg';
import { ref, watch, onMounted, onUnmounted } from 'vue';

const X_POS_CORRECTION = 0;
const VALUE_CHANGE_DELAY_MS = 50;

export default {
    name: 'slider-control',
    props: {
        currentValue: {
            type: [Number],
            default: 0,
            required: false
        },
        defaultValue: {
            type: [Number],
            default: 0,
            required: false
        },
        maxValue: {
            type: [Number],
            default: 100,
            required: false
        },
        minValue: {
            type: [Number],
            default: 0,
            required: false
        },
        applyImmediately: {
            type: [Boolean],
            default: true,
            required: false
        }
    },
    emits: ["change"],
    setup(props, ctx) {
        let movingIndicator = false;
        const maxValue = ref(props.maxValue).value;
        const minValue = ref(props.minValue).value;

        const indicator = ref(null);
        const indicatorWrapper = ref(null);
        const progressBorder = ref(null);
        const progress = ref(null);
        const sliderBar = ref(null);
        const sliderBarLeft = ref(null);
        const sliderBarRight = ref(null);
        const sliderValue = ref('sliderValue');

        watch(() => props.currentValue, async (val) => {
            if (!movingIndicator) {
                await isWindowReady();

                if (val === 0) {
                    const sliderBarLeftRef = sliderBarLeft.value;
                    sliderBarLeftRef.style.width = '0%';
                    sliderBarLeftRef.style.left = '0%';
                } else if (val > maxValue) {
                    val = maxValue;
                } else if (val < minValue) {
                    val = minValue;
                }

                setValueIndicatorPercentage((val - Number(minValue)) / (Number(maxValue) - Number(minValue)));
            }
        });

        const isWindowReady = async () => {
            while (progress.value.clientWidth === 0) {
                await new Promise(resolve => setTimeout(resolve, 100));
            }
        }

        const getSliderValue = () => {
            if (movingIndicator) {
                return getSelectedValue();
            }

            return getValueFromPercentage(sliderBarRight.value.style.width);
        };

        const getSelectedValue = () => {
            return getValueFromPercentage(indicatorWrapper.value.style.left);
        };

        const getValueFromPercentage = (percentage) => {
            const valuePercentage = parseFloat(percentage.slice(0, -1));
            return ((Number(maxValue) - Number(minValue)) / 100 * valuePercentage + Number(minValue)).toFixed() | 0;
        };

        const calculateNewXPos = (mouseXPos) => {
            const newX = mouseXPos - X_POS_CORRECTION - progressBorder.value.offsetLeft;
            return Math.min(Math.max(newX, 0), progress.value.clientWidth);
        };

        const colorPosition = (event) => {
            let xPos;
            if (event.type === 'touchstart' || event.type === 'touchmove') {
                let touch = event.touches[0];
                xPos = touch.pageX;
            } else {
                xPos = event.clientX;
            }

            const newX = calculateNewXPos(xPos);
            const positionInPercentage = newX / progress.value.clientWidth * 100;

            sliderValue.value.style.left = positionInPercentage + '%';

            if (!movingIndicator) {
                sliderBarRight.value.style.width = positionInPercentage + '%';

                let selectedValuePercentage = parseFloat(indicatorWrapper.value.style.left.slice(0, -1));
                if (!selectedValuePercentage) {
                    selectedValuePercentage = 0;
                }

                updateLeftBar(positionInPercentage, selectedValuePercentage, newX);
            }
            displaySliderValue();
        };

        const updateLeftBar = (positionInPercentage, selectedValuePercentage, newX) => {
            const sliderBarLeftRef = sliderBarLeft.value;
            sliderBarLeftRef.style.left = positionInPercentage + '%';

            if (positionInPercentage < selectedValuePercentage) {
                const clientWidth = progress.value.clientWidth;
                const width = clientWidth / 100 * selectedValuePercentage - newX;
                const widthPercentage = width / clientWidth * 100;
                sliderBarLeftRef.style.width = widthPercentage + '%';
            } else {
                sliderBarLeftRef.style.width = '0%';
            }
        };

        const displaySliderValue = () => {
            sliderValue.value.innerHTML = getSliderValue();
        };

        const setPosAndMove = (event) => {
            move(event);
            initMoveIndicator(event);
        };

        const stopMoveIndicator = () => {
            if (movingIndicator) {
                emitChange();
                movingIndicator = false;
            }

            indicator.value.classList?.remove('hover');
            sliderValue.value.classList?.remove('hover');
            removeEventListener('mousemove', move);
            removeEventListener('touchmove', move);
        };

        const initMoveIndicator = () => {
            addEventListener('mousemove', move, false);
            addEventListener('touchmove', move, false);
        };

        const move = (event) => {
            indicator.value.classList?.add('hover');
            sliderValue.value.classList?.add('hover');
            movingIndicator = true;

            let xPos;
            if (event.type === 'touchstart' || event.type === 'touchmove') {
                let touch = event.touches[0];
                xPos = touch.pageX;
            } else {
                xPos = event.clientX;
            }

            const newX = calculateNewXPos(xPos);
            const positionInPercentage = newX / progress.value.clientWidth * 100;

            resetSliderBars(positionInPercentage);
            setValueIndicatorInPixels(newX);

            displaySliderValue();

            if (props.applyImmediately) {
                emitChange();
            }
        };

        const resetSliderBars = (positionInPercentage) => {
            const pos = positionInPercentage + '%';
            sliderBarLeft.value.style.width = '0%';
            sliderBarLeft.value.style.left = pos;

            sliderBarRight.value.style.width = pos;
            sliderValue.value.style.left = pos;
        };

        const setValueIndicatorPercentage = (valuePercentage) => {
            if (valuePercentage < 1) {
                const posInPixels = progress.value.clientWidth * valuePercentage;
                setValueIndicatorInPixels(posInPixels);
            } else {
                setValueIndicatorInPixels(progress.value.clientWidth);
            }
        };

        const setValueIndicatorInPixels = (posInPixels) => {
            const barWidth = progress.value.clientWidth;
            indicatorWrapper.value.style.left = posInPixels / barWidth * 100 + '%';
            sliderBar.value.style.width = posInPixels / barWidth * 100 + '%';
            sliderBarLeft.value.style.width = posInPixels - sliderBarLeft.value.offsetLeft + 'px';
        };

        const setDefaultValue = () => {
            setValueIndicatorPercentage((props.defaultValue - Number(minValue)) / (Number(maxValue) - Number(minValue)));
            emitChange();
        };

        const throttle = (fn, limit) => {
            let throttleTimer;
            let lastRan = Date.now();
            return (...args) => {
                clearTimeout(throttleTimer);
                throttleTimer = setTimeout(() => {
                    const now = Date.now();
                    if ((now - lastRan) >= limit) {
                        fn.apply(this, args);
                        lastRan = now;
                    }
                }, limit - (Date.now() - lastRan));
            };
        };

        const emitChange = throttle(() => {
            ctx.emit('change', getSelectedValue());
        }, VALUE_CHANGE_DELAY_MS);

        const handleKeyDown = (event) => {
            switch (event.code) {
                case 'Home': {
                    setValueIndicatorPercentage((Number(minValue) - Number(minValue)) / (Number(maxValue) - Number(minValue)));
                    emitChange();

                    event.preventDefault();
                    break;
                }
                case 'End': {
                    setValueIndicatorPercentage((Number(maxValue) - Number(minValue)) / (Number(maxValue) - Number(minValue)));
                    emitChange();

                    event.preventDefault();
                    break;
                }
                case 'Delete': {
                    setDefaultValue();

                    event.preventDefault();
                    break;
                }
                case 'ArrowUp':
                case 'ArrowLeft': {
                    const value = getSelectedValue() - 1;

                    if (value >= Number(minValue)) {
                        setValueIndicatorPercentage((value - Number(minValue)) / (Number(maxValue) - Number(minValue)));
                        emitChange();
                    }

                    event.preventDefault();
                    break;
                }
                case 'ArrowDown':
                case 'ArrowRight': {
                    const value = getSelectedValue() + 1;

                    if (value <= Number(maxValue)) {
                        setValueIndicatorPercentage((value - Number(minValue)) / (Number(maxValue) - Number(minValue)));
                        emitChange();
                    }

                    event.preventDefault();
                    break;
                }

                case 'PageUp': {
                    const value = getSelectedValue() - 10;

                    if (value >= Number(minValue)) {
                        setValueIndicatorPercentage((value - Number(minValue)) / (Number(maxValue) - Number(minValue)));
                        emitChange();
                    }

                    event.preventDefault();
                    break;
                }
                case 'PageDown': {
                    const value = getSelectedValue() + 10;

                    if (value <= Number(maxValue)) {
                        setValueIndicatorPercentage((value - Number(minValue)) / (Number(maxValue) - Number(minValue)));
                        emitChange();
                    }

                    event.preventDefault();
                    break;
                }
            }
        };

        onMounted(() => {
            addEventListener('mouseup', stopMoveIndicator, false);
            addEventListener('touchend', stopMoveIndicator, false);
        });

        onUnmounted(() => {
            removeEventListener('mouseup', stopMoveIndicator);
            removeEventListener('touchend', stopMoveIndicator);
        });

        return {
            indicator,
            indicatorWrapper,
            progress,
            progressBorder,
            sliderBar,
            sliderBarLeft,
            sliderBarRight,
            sliderValue,
            colorPosition,
            handleKeyDown,
            initMoveIndicator,
            setPosAndMove,
            setDefaultValue,
        }
    },
    components: {
        Indicator
    }
};

</script>

<style scoped>

.progress-wrapper {
    padding: 8px 0;
    width: 100%;
    position: relative;
}

.progress-bar {
    padding: 8px 0;
}

.indicator-wrapper {
    width: 12px;
    height: 12px;
    display: flex;
    flex-direction: column;

    align-items: center;
    justify-content: center;
    position: absolute;
}

.progress-bar:hover:focus-visible .slider-value {
    background-color: rgba(36, 36, 68, 1);
    opacity: 1 !important;
}

.slider-value {
    position: absolute;
    display: flex;
    flex-direction: row;
    text-align: center;
    justify-content: center;

    width: 44px;
    height: 19px;

    top: 1px;
    margin-left: -25px;

    color: #CBCBD0;
    text-shadow: 0 0 2px #000000, 0 0 4px #000000;

    opacity: 0;
    background-color: rgba(16, 16, 48, 0.9);
    padding: 0 0 0 0;
    border-radius: 6px;

    border-color: rgba(76, 76, 94, 0.9);
    border-width: 1px;
    border-style: solid;

    transition: opacity 300ms ease-in, transform 0s ease-in 300ms;

    font-family: monospace;
    font-size: 16px;
}

.slider-value.hover, .progress-border:hover .slider-value {
    transform: translateY(-12px);
    transition: opacity 300ms ease-in;
    opacity: 0.7;
}

.indicator {
    width: 12px;
    height: 12px;
    position: relative;
    top: -3px;
    left: 0;
    padding: 5px;
    margin-left: -12px;
    z-index: 9999;

    display: flex;
    justify-content: center;

    transform: scale(0);
    transition: transform 100ms ease-out;
    text-align: center;
    overflow: visible;

    outline: none;

    fill: #ADABBC;
    filter: drop-shadow(0px 2px 2px rgb(0 0 0 / 0.4));
}

.indicator.hover, .progress-border:hover .indicator {
    transform: scale(1.2);
    z-index: 40;
}

.progress-bar:focus-visible .indicator {
    transform: scale(1.2);
    z-index: 40;
}

.progress-border {
    margin: 0;
    padding: 0;
    user-select: none;
    width: 100%;

    -webkit-tap-highlight-color: transparent;
}

.progress-border:hover {
    cursor: pointer;
}

.progress {
    width: 100%;
    height: 6px;
    position: relative;
    background-color: rgba(235, 235, 255, .18);
    overflow: hidden;
    display: flex;
    flex-direction: column;

    box-shadow: 0 0 0 1px rgba(50, 50, 80, .2);
}

.progress-border:hover .slider-bar-right, .progress-border:hover .slider-bar-left {
    opacity: 1;
}

.slider-bar {
    width: 0;
    height: 6px;
    position: absolute;
    background-color: #ADABBC;
    z-index: 30;
}

.slider-bar-left {
    opacity: 0;
    transition: opacity 300ms ease;
    width: 100%;
    height: 6px;
    position: absolute;
    background-color: #8583A5;
    z-index: 31;
}

.slider-bar-right {
    opacity: 0;
    transition: opacity 300ms ease;
    width: 100%;
    height: 6px;
    position: relative;
    background-color: rgba(210, 210, 225, .35);
    z-index: 20;
}

</style>
