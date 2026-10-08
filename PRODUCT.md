# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

The user approved Rust, WebAssembly, Bevy, Rapier, and a small static browser shell.

## Users

The user wants to drive a car through a desert in a local browser.

## Product Purpose

Recreate the selected driving loop from The Long Drive. The user accepted a recreation with original code and procedural assets.

## Operating Context

The game runs on localhost on a desktop. The user plays with a keyboard and mouse.

## Capabilities and Constraints

The approved scope includes car physics, an endless desert road, two cameras, fuel, and refueling stops.
It includes pause, focus handling, vehicle recovery, and restart.
The browser runs the simulation and rendering. It uses WebGL2 and WASM.
Walking, inventory, multiplayer, asset import, persistence, and public deployment are outside this build.

## Evidence on Hand

The user approved the design spec and requested implementation of the plan.
The local Steam installation uses Unity 2021.3.18f1 and Mono.
The research sources and installation findings are in the design spec.

## Product Principles

- Show the game before asking the player to start.
- Keep controls and fuel state readable over the scene.
- Build and verify each working layer.
- Preserve the approved scope.
