# SID-Device

SID-Device is a cross-platform Network SID Device that emulates
the Commodore 64 sound chip, a.k.a. SID chip (6581/8580).
It runs in a local network or on your own machine.

SID-Device can be used by SID-players like
[ACID 64 Player Pro](https://www.acid64.com), 
[ACID 64 console player](https://github.com/WilfredC64/acid64c),
[JSidplay2](https://sourceforge.net/projects/jsidplay2/) and
[USBSID-Player](https://github.com/LouDnl/USBSID-Player/releases).

SID-Device uses the emulation engine reSID 1.0 that is included in the Commodore 64 emulator Vice.

This project started with the goal to turn a Raspberry Pi
into a SID device.
You can connect your Raspberry Pi, with Raspberry Pi Desktop installed,
to your local network via cable or Wi-Fi and install SID-Device on it. 
Any SID player that supports the Network SID Interface
can then connect to the SID-Device and play SID tunes on it.

## Development

To build the source code you need to have the following tools installed: 
[Rust](https://www.rust-lang.org/),
[Node.js](https://nodejs.org/) and
[LLVM](https://llvm.org/). 
For the full prerequisites, follow the guideline of [Tauri](https://tauri.studio/v1/guides/getting-started/prerequisites).

For the first time you need to install all dependencies with:

```
npm install
```

Now you can build the project with:

```
npm run tauri build
```

## macOS Installation Note

The macOS build is ad-hoc signed but not signed with an Apple Developer ID
or notarized, so Gatekeeper will show an "unidentified developer" warning
the first time you open the app after downloading the `.dmg`. To run it:

1. Open the `.dmg` and drag `sid-device.app` to Applications (or wherever
   you prefer).
2. Right-click (or Control-click) `sid-device.app` and choose **Open**, then
   confirm **Open** in the dialog that appears, or
3. Remove the quarantine attribute from a terminal:
   ```
   xattr -d com.apple.quarantine /path/to/sid-device.app
   ```
You only need to do this once per downloaded copy.

## Documentation

For documentation about the network SID interface, see the
[Network SID Device V5](https://github.com/WilfredC64/acid64c/blob/master/docs/netsiddev.adoc).

## Thanks

Thanks to Dag Lem and all the team members and ex team members of Vice who
helped with the SID chip emulation.

Thanks to Nuke.YKT for the Yamaha FM OPL3 chip emulation.

Thanks to Ken H&auml;ndel and Antti S. Lankila for their contributions to developing
the network SID interface.

## Copyright

SID Device &ndash; Copyright &#xa9; 2021 - 2026 by Wilfred Bos

Network SID Interface &ndash; Copyright &#xa9; 2007 - 2026
by Wilfred Bos, Ken H&auml;ndel and Antti S. Lankila

reSID v1.0 &ndash; Copyright &#xa9; 1999 - 2010 by Dag Lem

Nuked-OPL3 &ndash; Copyright &#xa9; 2013 - 2020 by Nuke.YKT

## Licensing

The source code is licensed under the GPL v3 license. License is available [here](/LICENSE).
