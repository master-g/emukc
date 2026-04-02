# Crates

This directory contains the source code for the crates that make up the `EmuKC` project.

## `emukc_app`

This crate contains the application template and utilities to build an `EmuKC` application.

## `emukc_bootstrap`

This crate contains the bootstrap logic for volatile third-party resources, plus manual tooling for generating repo-tracked offline assets such as the wikiwiki map catalog.

## `emukc_cache`

This crate contains the cache implementation used by the `EmuKC` project.

## `emukc_crypto`

This crate contains the cryptographic primitives used by the `EmuKC` project.

Note that the cryptographic used by `EmuKC` is not intended to be secure, but rather to be simple and easy to use.

## `emukc_db`

This crate contains the database implementation used by the `EmuKC` project.

## `emukc_dylib`

This crate produces a dynamic library that can be used to speed up the build process of the `EmuKC` project.

## `emukc_internal`

One crate to `use` them all.

## `emukc_log`

This crate contains the logging utilities used by the `EmuKC` project.

## `emukc_macros`

This crate contains the procedural macros used by the `EmuKC` project.

## `emukc_model`

This crate contains the data model used by the `EmuKC` project.

## `emukc_network`

This crate contains the network utilities used by the `EmuKC` project.

## `emukc_time`

This crate contains the time utilities used by the `EmuKC` project.
