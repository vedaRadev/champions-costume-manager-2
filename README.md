<sub>Note: project/repo name is just a working title and may change in the future.</sub>
# Champions Costume Manager 2
An offline tool for managing in-game displays of costume save files in Champions
Online. This will initially be a command-line tool but the plan is to eventually
have a functional GUI serve as the primary form of user interaction.

![demo gif](./images/demo.gif)

Note that since Champions Online primarily targets Windows that is also
currently the primary target of this application. At the moment the command-line
tool _should_ work on Linux, though it is untested and some features may be
missing (e.g. file creation time copying). If there are enough people who play
Champions Online via Linux who also want to use this tool, I will see about
adding true cross-platform support. Until then, assume that upcoming GUI
features will be Windows-specific.

This tool is still early in development, so use at your own risk! If you're
paranoid and want to protect your saves, back them up! Save files can be found
at the following location:

```
<base game installation>/Champions Online/Live/screenshots
```

## Table of Contents
- [Why?](#why?)
- [Setup](#setup)
    - [Requirements](#requirements)
    - [Installation](#installation)
- [Usage](#usage)
- [Contributing](#contributing)

## Why?
Champions Online has an amazing character ("costume" in Champions parlance)
creator but little to no support for managing and organizing the hundreds of
saved costume files that any avid costumer will inevitably generate. To make
matters worse, the default in-game display of costume saves takes the form of
the unruly and often unhelpful `accountnameCharactername Date Time`. Costume
saves are comprised of costume-specific metadata embedded in JPEG image files,
and since the display names of save files in-game are derived from a combination
of this metadata the filename itself, deviating from the default format is a
difficult task for anyone without knowledge of the JPEG format and either a hex
editor or a programming language and image parsing library.

In short, as a starting point, this tool aims to provide a trivial way for users
to change the in-game displays of their saved costumes from this

![Champions Online in-game costume save list, before](./images/in-game-save-display-before.jpg)

to something much nicer and more maintainable:

![Champions Online in-game costume save list, after](./images/in-game-save-display-after.jpg)

## Setup
Currently the only way to run this tool is to compile the source yourself and
(optionally) add the resulting binary to your path. In the future, GitHub
releases will be utilized to distribute pre-compiled binaries.

### Requirements
- [Rust (1.84.0 is what I'm currently using) and Cargo](https://www.rust-lang.org/tools/install)

### Installation
1. Clone this repository.
2. Run `cargo b` (for development/debugging) or `cargo b --release`.
3. Add `./target/debug` or `./target/release` to your PATH (or move the
   executable to wherever you want first then add _that_ directory to your
   PATH).

Note: You can also just `cargo r -- <application arguments>` but this is fairly
cumbersome if you plan on actually using the tool. If you're actually developing
then I recommend adding the path to whatever test file you're using as an
environment variable.

## Usage
```
Usage: ccm.exe <costume save file path> [options]

-h, --help
    Show this usage information.

-c, --set-character-name <character_name>
    Set the character name that will be displayed in-game.

-a, --set-account-name <account_name>
    Set the account name that will be displayed in-game.

-s, --set-save-name <save_name>
    Set the portion of the filename between the "Costume_" prefix and the j2000
    timestamp suffix (if it exists).

-t, --strip-timestamp
    Strips the j2000 timestamp suffix from the save file name, removing the date
    display from its entry in the in-game save menu. If there is no j2000
    timestamp or if the timestamp is invalid, this is effectively a no-op.
    NOTE: If you want to re-add an in-game date display, you will need to
    calculate your own j2000 timestamp and append it to the end of the file name
    yourself in the form "_<timestamp>".

-i, --inspect [short|long]
    Print file name, in-game save display, and costume metadata. Defaults to
    short if no specification is supplied. Long will print the costume hash and
    proprietary costume specification as well as all the information that short
    displays.
    If this option is supplied in conjunction with mutative options such as
    --set-character-name or --strip-timestamp, the mutative options are applied
    first then the updated information is inspected and displayed.

--dry-run
    Applies mutative options to the in-memory costume save but does not write
    the results to disk. Use in conjunction with --inspect to see the results of
    potential changes. This option does not need to be specified if no mutative
    options are used.
```

## Contributing
This project isn't far enough along to accept contributions yet.
