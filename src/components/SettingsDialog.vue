<!--
  Copyright (C) 2022 - 2025 Wilfred Bos
  Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.
-->

<template>
    <div id="settings">
        <TitleBar />
        <div class="properties">
            <p>
                <select-box
                    :selectedIndex="config.audio_device_number"
                    :options="deviceList"
                    @change="changeAudioDevice"
                ></select-box>
            </p>
            <br/>
            <p class="slider-line">
                <span class="filter-label">6581 Filter Bias: {{config.filter_bias_6581}}</span>
                <slider-control
                    class="slider"
                    :current-value="config.filter_bias_6581"
                    :default-value="config.default_filter_bias_6581"
                    :min-value="-100"
                    :max-value="100"
                    @change="setFilter6581">
                </slider-control>
            </p>
            <br/>
            <p class="check-box-wrapper">
                <check-box
                    id="enable-digi-boost"
                    :checked="config.digiboost_enabled"
                    label="8580 Digi Boost"
                    @change="enableDigiBoost">
                </check-box>
            </p>
            <br/>
            <div class="bottom-settings">
                <div class="bottom-settings-wrapper">
                    <div>
                        <p class="check-box-wrapper">
                            <check-box
                                id="enable-external-ip"
                                :checked="config.allow_external_connections"
                                label="Allow external IP connections"
                                @change="allowExternalIp">
                            </check-box>
                        </p>
                        <br/>
                        <p class="check-box-wrapper">
                            <check-box
                                id="restart-at-startup"
                                :checked="config.launch_at_start_enabled"
                                label="Launch at startup"
                                @change="toggleLaunchAtStart">
                            </check-box>
                        </p>
                    </div>
                    <div class="settings-button"
                         tabindex="0"
                         @keyup="handleKeyUpResetDefault"
                         @click="resetToDefault">
                        Reset to default
                    </div>
                </div>
            </div>
        </div>
    </div>
</template>

<script>

import { emit, listen } from '@tauri-apps/api/event';
import { invoke } from '@tauri-apps/api/core'
import { ask } from '@tauri-apps/plugin-dialog';
import { ref } from 'vue'
import CheckBox from './CheckBox.vue'
import SelectBox from './SelectBox.vue'
import SliderControl from './SliderControl.vue'
import TitleBar from './TitleBar.vue';

export default {
    name: 'SettingsDialog',
    setup() {
        const deviceList = ref([]);
        const config = ref({});

        let deviceReady = false;

        const activateListeners = async () => {
            await listen('ready', async () => {
                deviceReady = true;
            });

            await listen('error', async (message) => {
                const answer = await ask(message.payload + '\r\rTry again?', {
                    title: 'SID-Device Error',
                    kind: 'info',
                });

                if (answer) {
                    await emit('retry');
                } else {
                    await emit('exit');
                }
            });
        }

        activateListeners();

        const isDeviceReady = async () => {
            deviceReady = false;

            do {
                await new Promise(resolve => setTimeout(resolve, 200));
                await emit('device-ready');
            } while (!deviceReady);
        };

        isDeviceReady();

        const refreshDeviceList = () => {
            invoke('get_devices_cmd').then((response) => {
                deviceList.value = [
                    'Default Sound Driver: ' + response.devices[response.default_device],
                    ...response.devices
                ];

                if (config.value.audio_device_number == null || config.value.audio_device_number >= deviceList.value.length) {
                    config.value.audio_device_number = 0;
                    invoke('change_audio_device_cmd', {deviceIndex: 0});
                }
            });
        }

        const setConfig = (newConfig) => {
            config.value = newConfig;
            if (config.value.audio_device_number != null) {
                config.value.audio_device_number++;
            } else {
                config.value.audio_device_number = 0;
            }
            refreshDeviceList();
        }

        invoke('get_config_cmd').then(config => {
            setConfig(config);
        });

        const resetToDefault = () => {
            invoke('reset_to_default_cmd');
        };

        const changeAudioDevice = (deviceId) => {
            config.value.audio_device_number = Number(deviceId);
            invoke('change_audio_device_cmd', {deviceIndex: Number(deviceId)});
        };

        const toggleLaunchAtStart = (event) => {
            config.value.launch_at_start_enabled = event.target.checked;
            invoke('toggle_launch_at_start_cmd');
        };

        const enableDigiBoost = (event) => {
            const enabled = event.target.checked;
            config.value.digiboost_enabled = enabled;
            invoke('enable_digiboost_cmd', {digiBoostEnabled: enabled});
        };

        const allowExternalIp = (event) => {
            const enabled = event.target.checked;
            config.value.allow_external_connections = enabled;
            invoke('allow_external_ip_cmd', {externalIpAllowed: enabled});

            isDeviceReady();
        };

        const setFilter6581 = (filterValue) => {
            config.value.filter_bias_6581 = filterValue;
            invoke('change_filter_bias_6581_cmd', {filterBias6581: filterValue});
        };

        const handleKeyUpResetDefault = (event) => {
            switch (event.code) {
                case 'Space': {
                    resetToDefault();
                    event.preventDefault();
                    break;
                }
            }
        }

        return {
            config,
            deviceList,
            allowExternalIp,
            changeAudioDevice,
            enableDigiBoost,
            toggleLaunchAtStart,
            handleKeyUpResetDefault,
            resetToDefault,
            setFilter6581,
            setConfig
        }
    },
    components: {
        CheckBox,
        SelectBox,
        SliderControl,
        TitleBar
    }
}

</script>

<style scoped>

#settings {
    background-color: black;
    background-image: url("../assets/8580_SID_600x450.jpg");
    background-repeat: no-repeat;
    background-size: cover;
    background-position: bottom center;
    background-attachment: inherit;
    height: 100%;
    color: #d4d0e0;
}

.properties {
    padding: 42px 20px 30px 20px;
}

.bottom-settings {
    bottom: 0;
    left: 0;
    position: absolute;
    width: 100%;
}

.bottom-settings-wrapper {
    position: relative;
    padding: 20px 20px;
    display: flex;
    flex-direction: row;
    justify-content: space-between;
    align-items: flex-end;
}

.check-box-wrapper {
    display: flex;
    align-items: center;
    height: 22px;
}

.slider-line {
    height: 22px;
    display: flex;
    flex-direction: row;
    align-items: center;
    justify-content: space-between;
}

.filter-label {
    white-space: nowrap;
    min-width: 170px;
}

.settings-button {
    width: 150px;
    height: 34px;
    text-align: center;
    display: flex;
    flex-direction: column;
    justify-content: center;
    border-radius: 10px;
    background-color: rgba(6, 6, 38, 0.5);
    border: 1px solid rgba(96, 96, 138, 0.5);
    color: #b4b0c0;
    cursor: pointer;
}

.settings-button:hover {
    color: #d4d0e0;
    background-color: rgba(16, 16, 48, 0.5);
}

.settings-button:focus-visible {
    color: #d4d0e0;
    background-color: rgba(16, 16, 48, 0.5);
}

</style>
