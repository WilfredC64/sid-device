// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

#pragma once
#include <cstdint>
#include <memory>
#include "resid10/sid.h"
#include "rust/cxx.h"

namespace resid_bridge {
    std::unique_ptr<reSID::SID> new_sid();
    void set_chip_model(reSID::SID& sid, uint32_t model);
    void set_voice_mask(reSID::SID& sid, uint32_t mask);
    void enable_filter(reSID::SID& sid, bool enable);
    void adjust_filter_bias(reSID::SID& sid, double dac_bias);
    void enable_external_filter(reSID::SID& sid, bool enable);
    bool set_sampling_parameters(reSID::SID& sid, double clock_freq, uint32_t method, double sample_freq, double pass_freq, double filter_scale);
    void adjust_sampling_frequency(reSID::SID& sid, double sample_freq);
    void clock(reSID::SID& sid);
    void clock_delta(reSID::SID& sid, int32_t delta_t);
    int32_t clock_buffer(reSID::SID& sid, int32_t& delta_t, rust::Slice<int16_t> buf, int32_t interleave);
    void reset(reSID::SID& sid);
    uint32_t read(reSID::SID& sid, uint32_t reg);
    void write(reSID::SID& sid, uint32_t reg, uint32_t value);
    void input(reSID::SID& sid, int16_t sample);
}

