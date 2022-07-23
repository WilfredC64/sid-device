//  ---------------------------------------------------------------------------
//  This file is part of reSID, a MOS6581 SID emulator engine.
//  Copyright (C) 1999  Dag Lem <resid@nimrod.no>
//
//  This program is free software; you can redistribute it and/or modify
//  it under the terms of the GNU General Public License as published by
//  the Free Software Foundation; either version 2 of the License, or
//  (at your option) any later version.
//
//  This program is distributed in the hope that it will be useful,
//  but WITHOUT ANY WARRANTY; without even the implied warranty of
//  MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
//  GNU General Public License for more details.
//
//  You should have received a copy of the GNU General Public License
//  along with this program; if not, write to the Free Software
//  Foundation, Inc., 59 Temple Place, Suite 330, Boston, MA  02111-1307  USA
//  ---------------------------------------------------------------------------

#ifndef RESID_SIDDEFS_H
#define RESID_SIDDEFS_H

// Define bool, true, and false for C++ compilers that lack these keywords.
#define RESID_HAVE_BOOL 1

#ifndef NEW_8580_FILTER
#define NEW_8580_FILTER 1
#endif

#define HAVE_LOG1P 0
#define HAS_LOG1P 0

#if HAVE_LOG1P
#define HAS_LOG1P
#endif

// Inlining on/off.
#define RESID_INLINING 1
#define RESID_INLINE inline

// Support namespace

#ifdef RESID_NAMESPACE
#   define RESID_NAMESPACE_START \
    namespace RESID_NAMESPACE    \
    {
#   define RESID_NAMESPACE_STOP  \
    }
#else
#   define RESID_NAMESPACE_START
#   define RESID_NAMESPACE_STOP
#endif

// Branch prediction macros, lifted off the Linux kernel.
#if RESID_BRANCH_HINTS && HAVE_BUILTIN_EXPECT
#define likely(x)      __builtin_expect(!!(x), 1)
#define unlikely(x)    __builtin_expect(!!(x), 0)
#else
#define likely(x)      (x)
#define unlikely(x)    (x)
#endif

RESID_NAMESPACE_START

#if !RESID_HAVE_BOOL
typedef int bool;
const bool true = 1;
const bool false = 0;
#endif

// We could have used the smallest possible data type for each SID register,
// however this would give a slower engine because of data type conversions.
// An int is assumed to be at least 32 bits (necessary in the types reg24,
// cycle_count, and sound_sample). GNU does not support 16-bit machines
// (GNU Coding Standards: Portability between CPUs), so this should be
// a valid assumption.

namespace reSID {

typedef unsigned int reg4;
typedef unsigned int reg8;
typedef unsigned int reg12;
typedef unsigned int reg16;
typedef unsigned int reg24;

typedef int cycle_count;
typedef short short_point[2];
typedef double double_point[2];

enum chip_model { MOS6581, MOS8580 };

enum sampling_method { SAMPLE_FAST, SAMPLE_INTERPOLATE,
		       SAMPLE_RESAMPLE, SAMPLE_RESAMPLE_FASTMEM };
}

extern "C"
{
#ifndef RESID_VERSION_CC
extern const char* resid_version_string;
#else
const char* resid_version_string = VERSION;
#endif
}

RESID_NAMESPACE_STOP

#endif // not RESID_SIDDEFS_H
