# Rocket Factory (Archived Design Note)

> **Archived:** This early brainstorm has been superseded by `README.md` and `STEPPED_IMPLEMENTATION_PLAN.md`. The application was rebranded to **Rocket Bench (`rktb`)** as a tool-agnostic architecture planning TUI & CLI (`.rocket/` workspace files + external command-based adapters).

## Original Concept

A development planning platform built on TUI (`ratatui`), providing a minimalist text and vector-graphic interface for planning apps and components.

## Interface

* Layout: Three columns (Component list, Snap Grid canvas, Component Details) with header bar and footer keybind bar
* Keyboard-only interface with modal editors and theme switching
* Components/Pieces: Cards with name, status, and brief description, connectable by vector edges

## Function

* Quickly scaffold and design applications and components
* Push components/features to and pull task status from external tools via configurable command-based adapters
* Store component and feature metadata in `.rocket/` TOML and Markdown files

## Aesthetics

* Minimalist retro-futuristic appearance
* Themes stored in RON files (default: CRT amber and black)
