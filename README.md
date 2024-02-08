# komootgpx | [![Tests](https://img.shields.io/github/actions/workflow/status/cdown/komootgpx/ci.yml?branch=master)](https://github.com/cdown/komootgpx/actions?query=branch%3Amaster)

komootgpx creates a GPX from Komoot, even if you don't have the region
unlocked. It uses the data used to draw the map, and then puts that into a GPX
directly.

## Installation

    cargo install komootgpx

## Usage

By default the GPX is written to stdout, but you can also put it in a file. For
example:

    komootgpx 'https://www.komoot.com/smarttour/15628660' -o stockholm-loop.gpx
