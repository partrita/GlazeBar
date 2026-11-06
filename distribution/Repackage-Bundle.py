"""Update a verified architecture bundle with GlazeBar and, optionally, a rebuilt modified GlazeWM.

Dependency vendors, licenses and the official GlazeWM MSI are always retained.
Without --wm-runtime the modified GlazeWM from the base bundle is kept as well.
"""
import argparse
import hashlib
import json
from pathlib import Path
import struct
import subprocess
import zipfile


def sha(data):
    return hashlib.sha256(data).hexdigest().upper()


def replace_modified_wm(args, folder, repo, arch):
    """Swap in a rebuilt modified GlazeWM with its matching source, patch and notes."""
    machine = {'arm64': 0xAA64, 'x64': 0x8664}[arch]
    target = {'arm64': 'aarch64', 'x64': 'x86_64'}[arch] + '-pc-windows-msvc'
    for name in ['glazewm.exe', 'glazewm-watcher.exe']:
        data = (args.wm_runtime / name).read_bytes()
        if struct.unpack_from('<H', data, struct.unpack_from('<I', data, 60)[0] + 4)[0] != machine:
            raise RuntimeError('Wrong GlazeWM binary architecture: ' + name)
    toolchain = subprocess.check_output(['rustc', '+nightly', '--version'], text=True).strip()
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=args.wm_source).decode().split('\0')[:-1]
    names = sorted(set(tracked + ['.cargo/config.toml']))
    source = folder / 'source/glazewm-modified-source.zip'
    with zipfile.ZipFile(source) as old:
        if old.read('Cargo.lock').replace(b'\r\n', b'\n') != (args.wm_source / 'Cargo.lock').read_bytes().replace(b'\r\n', b'\n'):
            raise RuntimeError('Cannot reuse GlazeWM vendors with a different lockfile')
    with zipfile.ZipFile(source, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as archive:
        for name in names:
            if (args.wm_source / name).is_file():
                archive.write(args.wm_source / name, name)
    for name in ['glazewm.exe', 'glazewm-watcher.exe']:
        (folder / 'payload/wm' / name).write_bytes((args.wm_runtime / name).read_bytes())
    patch = repo / 'glazewm-patches/glazewm-v3.10.1.patch'
    (folder / 'source/glazewm-v3.10.1.patch').write_bytes(patch.read_bytes())
    (folder / 'source/GLAZEWM-MODIFICATIONS.md').write_bytes((repo / 'docs/glazewm-modifications.md').read_bytes())
    # Provenance files from earlier previews describe WM binaries that are no longer in the bundle.
    for stale in ['BUILD-PREVIEW4.json', 'BUILD-PREVIEW5.json']:
        (folder / 'source' / stale).unlink(missing_ok=True)
    (folder / 'source/BUILD-MODIFIED-WM.txt').write_text(
        'Extract glazewm-modified-source.zip into a build folder.\n'
        'Extract glazewm-vendor.zip INTO its vendor/ subfolder (beside Cargo.toml).\n'
        'Run in an MSVC developer shell for the target architecture (Windows SDK required):\n'
        f'set VERSION_NUMBER={args.wm_version}\n'
        f'cargo +nightly build --offline --locked --release --target {target} -p wm --bin glazewm -p wm-watcher --bin glazewm-watcher\n'
        f'Built with: {toolchain}\n', encoding='utf-8')
    return {
        'glazewmUpstream': 'https://github.com/glzr-io/glazewm/tree/v3.10.1',
        'modifiedWmSourceSHA256': sha(source.read_bytes()), 'wmPatchSHA256': sha(patch.read_bytes()),
        'wmSHA256': sha((args.wm_runtime / 'glazewm.exe').read_bytes()),
        'wmWatcherSHA256': sha((args.wm_runtime / 'glazewm-watcher.exe').read_bytes()),
        'wmToolchain': toolchain,
    }


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--base', type=Path, required=True)
    parser.add_argument('--bar', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    parser.add_argument('--version', default='0.0.9')
    parser.add_argument('--slim', action='store_true', help='create a slim runtime-only bundle without source and vendor archives')
    parser.add_argument('--wm-runtime', type=Path, help='folder with the rebuilt glazewm.exe and glazewm-watcher.exe')
    parser.add_argument('--wm-source', type=Path, help='git checkout of the modified GlazeWM source')
    parser.add_argument('--wm-version', help='VERSION_NUMBER the modified GlazeWM was built with')
    args = parser.parse_args()
    if bool(args.wm_runtime) != bool(args.wm_source) or bool(args.wm_runtime) != bool(args.wm_version):
        parser.error('--wm-runtime, --wm-source and --wm-version must be given together')
    repo = Path(__file__).resolve().parents[1]
    with zipfile.ZipFile(args.base) as base:
        manifest = json.loads(base.read('manifest.json').decode('utf-8-sig'))
        arch = manifest['architecture']
        for item in manifest['files']:
            if sha(base.read(item['path'])) != item['sha256'].upper():
                raise RuntimeError('Base integrity failed: ' + item['path'])
        binary = args.bar.read_bytes()
        machine = struct.unpack_from('<H', binary, struct.unpack_from('<I', binary, 60)[0] + 4)[0]
        if machine != {'arm64': 0xAA64, 'x64': 0x8664}[arch]:
            raise RuntimeError('Wrong binary architecture')
        folder = args.output / f'GlazeBar-Community-{args.version}-windows-{arch}'
        folder.mkdir(parents=True, exist_ok=False)
        base.extractall(folder)
    tracked = subprocess.check_output(['git', 'ls-files', '-z'], cwd=repo).decode().split('\0')[:-1]
    source = folder / 'source/glazebar-source.zip'
    replacement = source.with_suffix('.new.zip')
    with zipfile.ZipFile(source) as old, zipfile.ZipFile(replacement, 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as new:
        if old.read('native-rust/Cargo.lock').replace(b'\r\n', b'\n') != (repo / 'native-rust/Cargo.lock').read_bytes().replace(b'\r\n', b'\n'):
            raise RuntimeError('Cannot reuse vendors with a different lockfile')
        written = set()
        for entry in old.infolist():
            if entry.is_dir():
                continue
            name = entry.filename
            local = repo / name
            data = local.read_bytes() if name in tracked else old.read(name)
            if name == 'native-rust/.cargo/config.toml':
                data = local.read_bytes() + b"\n[source.crates-io]\nreplace-with = 'vendored-sources'\n[source.vendored-sources]\ndirectory = 'vendor'\n"
            new.writestr(name, data)
            written.add(name)
        for name in tracked:
            if name not in written:
                new.write(repo / name, name)
    replacement.replace(source)
    for name in ['GlazeBar.exe', 'GlazeTheme.exe']:
        (folder / 'payload' / name).write_bytes(binary)
    for name in ['Install.ps1', 'Uninstall.ps1', 'README.txt', 'README.ko.txt']:
        (folder / name).write_bytes((repo / 'distribution' / name).read_bytes())
    unchanged = ['modified GlazeWM', 'official GlazeWM MSI', 'dependency vendors', 'third-party licenses']
    wm_provenance = {}
    if args.wm_runtime:
        wm_provenance = replace_modified_wm(args, folder, repo, arch)
        manifest['glazewmVersion'] = args.wm_version
        unchanged = unchanged[1:]
    provenance = {
        'version': args.version, 'architecture': arch,
        'baseBundle': args.base.name, 'baseBundleSHA256': sha(args.base.read_bytes()),
        'barSourceRevision': subprocess.check_output(['git', 'rev-parse', 'HEAD'], cwd=repo).decode().strip(),
        'barSHA256': sha(binary), 'glazewmVersion': manifest['glazewmVersion'],
        'unchangedComponents': unchanged, **wm_provenance,
    }
    (folder / 'source/BUILD-GLAZEBAR.json').write_text(json.dumps(provenance, indent=2), encoding='utf-8')
    if args.slim:
        import shutil
        source_dir = folder / 'source'
        if source_dir.exists():
            for item in list(source_dir.glob('*.zip')):
                item.unlink(missing_ok=True)
            for item in list(source_dir.glob('*.tar*')):
                item.unlink(missing_ok=True)
        upstream_dir = folder / 'upstream'
        if upstream_dir.exists():
            shutil.rmtree(upstream_dir, ignore_errors=True)
            manifest.pop('glazewmMsi', None)
        for sub in ['glazebar', 'glazewm']:
            lic_sub = folder / 'licenses' / sub
            if lic_sub.exists():
                shutil.rmtree(lic_sub, ignore_errors=True)
    manifest['version'] = args.version
    manifest['files'] = [{'path': p.relative_to(folder).as_posix(), 'sha256': sha(p.read_bytes())}
                         for p in sorted(folder.rglob('*')) if p.is_file() and p.name != 'manifest.json']
    (folder / 'manifest.json').write_text(json.dumps(manifest, indent=2), encoding='utf-8')
    with zipfile.ZipFile(str(folder) + '.zip', 'w', zipfile.ZIP_DEFLATED, strict_timestamps=False) as archive:
        for p in sorted(folder.rglob('*')):
            if p.is_file():
                archive.write(p, p.relative_to(folder).as_posix())
    print(folder)


if __name__ == '__main__':
    main()
