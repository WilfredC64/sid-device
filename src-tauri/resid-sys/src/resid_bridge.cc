// Copyright (C) 2026 Wilfred Bos
// Licensed under the GNU GPL v3 license. See the LICENSE file for the terms and conditions.

#include "resid_bridge.h"

namespace resid_bridge {

std::unique_ptr<reSID::SID> new_sid() {
    return std::make_unique<reSID::SID>();
}

void set_chip_model(reSID::SID& sid, uint32_t model) {
    sid.set_chip_model(static_cast<reSID::chip_model>(model));
}

void set_voice_mask(reSID::SID& sid, uint32_t mask) {
    sid.set_voice_mask(static_cast<reSID::reg4>(mask));
}

void enable_filter(reSID::SID& sid, bool enable) {
    sid.enable_filter(enable);
}

void adjust_filter_bias(reSID::SID& sid, double dac_bias) {
    sid.adjust_filter_bias(dac_bias);
}

void enable_external_filter(reSID::SID& sid, bool enable) {
    sid.enable_external_filter(enable);
}

bool set_sampling_parameters(reSID::SID& sid, double clock_freq, uint32_t method, double sample_freq, double pass_freq, double filter_scale) {
    return sid.set_sampling_parameters(clock_freq, static_cast<reSID::sampling_method>(method), sample_freq, pass_freq, filter_scale);
}

void adjust_sampling_frequency(reSID::SID& sid, double sample_freq) {
    sid.adjust_sampling_frequency(sample_freq);
}

void clock(reSID::SID& sid) {
    sid.clock();
}

void clock_delta(reSID::SID& sid, int32_t delta_t) {
    reSID::cycle_count d = static_cast<reSID::cycle_count>(delta_t);
    sid.clock(d);
}

int32_t clock_buffer(reSID::SID& sid, int32_t& delta_t, rust::Slice<int16_t> buf, int32_t interleave) {
    reSID::cycle_count d = static_cast<reSID::cycle_count>(delta_t);
    int result = sid.clock(d, buf.data(), static_cast<int>(buf.size()), interleave);
    delta_t = static_cast<int32_t>(d);
    return static_cast<int32_t>(result);
}

void reset(reSID::SID& sid) {
    sid.reset();
}

uint32_t read(reSID::SID& sid, uint32_t reg) {
    return static_cast<uint32_t>(sid.read(static_cast<reSID::reg8>(reg)));
}

void write(reSID::SID& sid, uint32_t reg, uint32_t value) {
    sid.write(static_cast<reSID::reg8>(reg), static_cast<reSID::reg8>(value));
}

void input(reSID::SID& sid, int16_t sample) {
    sid.input(static_cast<short>(sample));
}

} // namespace resid_bridge

