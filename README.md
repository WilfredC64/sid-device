# SID-Device

SID-Device is a cross-platform Network SID Device that emulates
the Commodore 64 sound chip, a.k.a. SID chip (6581/8580).
It runs in a local network or on your own machine.

SID-Device can be used by SID-players like
[ACID 64 Player Pro](https://www.acid64.com), 
[ACID 64 console player](https://github.com/WilfredC64/acid64c) and
[JSidplay2](https://sourceforge.net/projects/jsidplay2/).

SID-Device v1.0 uses the emulation engine reSID 1.0 that is included in the Commodore 64 emulator Vice.

This project started with the goal to turn a Raspberry Pi
into a SID device.
You can connect your Raspberry Pi, with Raspberry Pi Desktop installed,
to your local network via cable or Wi-fi and install SID-Device on it. 
Any SID player that supports the Network SID Interface
can then connect to the SID-Device and play SID tunes on it.

## Development

To build the source code you need to install 
[Rust](https://www.rust-lang.org/) and
[NodeJS](https://nodejs.org/). 
For the full prerequisites, follow the guideline of [Tauri](https://tauri.studio/v1/guides/getting-started/prerequisites).

For the first time you need to install all dependencies with:

```
npm install
```

Now you can build the project with:

```
npm run tauri build
```

If you want to build the application on Raspberry Pi, make sure to change the "targets" property value "all" to "deb" in file tauri.conf.json.


## Documentation

For documentation about the network SID interface, see the
[Network SID Device V4](https://htmlpreview.github.io/?https://github.com/WilfredC64/acid64c/blob/master/docs/network_sid_device_v4.html) specification,
converted from the
[JSidplay2](https://sourceforge.net/p/jsidplay2/code/HEAD/tree/trunk/jsidplay2/src/main/asciidoc/netsiddev.adoc) project.


## Thanks

Thanks to Dag Lem and all the team members and ex team members of Vice who
helped with the SID chip emulation.

Thanks to Ken H&auml;ndel and Antti S. Lankila for creating the network SID interface that is used in JSidplay2 and JSidDevice.


## Copyright

SID Device v1.0 &ndash; Copyright &#xa9; 2021 - 2022 by Wilfred Bos

Network SID Interface &ndash; Copyright &#xa9; 2007 - 2022
by Wilfred Bos, Ken H&auml;ndel and Antti S. Lankila

reSID v1.0 &ndash; Copyright &#xa9; 1998 - 2022 by Dag Lem


## Licensing

The source code is licensed under the GPL v3 license. License is available [here](/LICENSE).
