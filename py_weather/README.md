# Python Weather Data

A Python GUI with PyO3 bindings to the Rust weather data API.

## Overview

The original weather data project was written in pure Python.
Once I had a Rust port of the weather data project I thought
it would be fun to build a Python GUI front-end with bindings
to the Rust weather data API.

There are two components to the Python project.

### py_lib

py_lib is a Rust based project. It uses PyO3 to implement the
Python weather data API and data conversions between the
Python runtime and native backend.

The result of a successful build is a wheel that can be
installed into a Python environment.

The PyO3 bindings must be built before installing the py_gui
project.

### py_gui

py_gui is a Python 3.11 based implementation of the weather
data GUI. It uses the py_lib bindings to access the weather
data API.

The result of a successful installation is an executable
that will launch the GUI.

## Installation

Use the following commands to bootstrap the Python virtual
environment.

```
C: python3 -m venv venv
C: venv\Scripts\Activate
(venv) C: python -m pip install --upgrade pip
(venv) C: pip install maturin
```
