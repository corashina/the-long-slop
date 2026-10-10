"""Current source archive and catalog records."""
from dataclasses import dataclass
from pathlib import Path

SCHEMA_VERSION = 1
APP_ID = 1017180
BUILD_ID = 11140890
UNITY_VERSION = '2021.3.18f1'
DATA_DIRECTORY = 'TheLongDrive_Data'
# This Steam build contains a built-in file authored with an earlier editor.
SERIALIZED_VERSIONS = {'Resources/unity default resources': '2021.3.13f1'}


def expected_serialized_version(container: str) -> str:
    return SERIALIZED_VERSIONS.get(container, UNITY_VERSION)


@dataclass(frozen=True)
class Snapshot:
    root: Path
    manifest_path: Path


@dataclass
class Catalog:
    schema_version: int
    app_id: int
    build_id: int
    unity_version: str
    containers: list[dict]
    streams: list[dict]
    objects: list[dict]
