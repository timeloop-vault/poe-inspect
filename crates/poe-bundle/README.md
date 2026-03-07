# Path of Exile Bundle Reader

Reads compressed bundles for the game Path of Exile created by Grinding Gear Games.

## Table of Contents

- [Background](#background)
- [Building](#building)
- [Usage](#usage)
- [License](#license)

## Background

As of patch 3.11.2 Grinding Gear Games began using a new way of storing game files. Some files are now compressed in bundles using [RAD Game Tools](http://www.radgametools.com) proprietary [Oodle Compression](http://www.radgametools.com/oodle.htm) suite.

## Building


This project uses [this](https://github.com/daniel-dimovski/ooz) fork of ooz (an open source imlementation of Oodle).

Cargo will look for libooz in the root of the repository, you can change this in the `build.rs` file.

The usual cargo commands work.


```sh
$ cargo build --release
```

## Usage

There is not CLI implemented yet, as my initial use case only requires needs a lib.
