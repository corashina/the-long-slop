"""Index object identities and original serialized byte ranges."""
import json
from collections import Counter

from .archive import verify_snapshot
from .types import APP_ID, BUILD_ID, UNITY_VERSION, SCHEMA_VERSION, DATA_DIRECTORY, Catalog, Snapshot
from ..source import Source, source_id


def inventory_source(source: Source, snapshot: Snapshot) -> Catalog:
    if source.root.resolve() != snapshot.root.resolve():
        raise ValueError('Inventory source differs from the verified archive')
    verify_snapshot(snapshot)
    manifest = json.loads(snapshot.manifest_path.read_text(encoding='utf-8'))
    files = {record['path']: record for record in manifest['files']}
    objects = []
    counts = Counter()
    for reader in source.objects.values():
        container = source.container_path(reader)
        file = files[f'{DATA_DIRECTORY}/{container}']
        if reader.byte_start < 0 or reader.byte_size < 0 or reader.byte_start + reader.byte_size > file['size']:
            raise ValueError(f'Object range escapes source file: {container}:{reader.path_id}')
        objects.append({'source': source_id(reader), 'class_id': reader.class_id, 'type': reader.type.name,
                        'byte_start': reader.byte_start, 'byte_size': reader.byte_size})
        counts[container] += 1
    objects.sort(key=lambda item: (item['source']['file'], item['source']['path_id']))
    containers = []
    for name, asset in sorted(source.serialized_files.items()):
        record = files[f'{DATA_DIRECTORY}/{name}']
        containers.append({'path': name, 'size': record['size'], 'sha256': record['sha256'],
                           'unity_version': asset.unity_version, 'object_count': counts[name],
                           'externals': [{'file_id': index, 'path': item.path,
                                          'guid': item.guid.hex() if item.guid else None, 'type': item.type}
                                         for index, item in enumerate(asset.externals, 1)]})
    streams = [{'path': name, 'size': files[f'{DATA_DIRECTORY}/{name}']['size'],
                'sha256': files[f'{DATA_DIRECTORY}/{name}']['sha256']} for name in source.resource_streams]
    return Catalog(SCHEMA_VERSION, APP_ID, BUILD_ID, UNITY_VERSION, containers, streams, objects)
