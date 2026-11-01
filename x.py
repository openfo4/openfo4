#!/usr/bin/python3

import argparse
import os
import shutil
import subprocess
import stat

# 755 permissions
RWX_PERMS = stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH

def main():
    args = Args.parse()

    cwd = os.getcwd()
    print(f"executing in {cwd}")

    rustflags = []
    features = []

    if args.embed_shaders:
        rustflags.append("openfo4_embed_shaders")

    if args.tracy:
        features.append("tracy")

    cargo_build("openfo4_client", rustflags, features)
    os.makedirs("build", exist_ok=True)

    shutil.copyfile(with_exe_suffix("target/release/openfo4_client"), with_exe_suffix("build/openfo4_client"))
    os.chmod(with_exe_suffix("build/openfo4_client"), RWX_PERMS)

def run_command(args, cwd=None, env=None):
    cmd = " ".join(args)
    print(f"running {cmd}")

    if env is not None:
        env = {**os.environ, **env}

    handle = subprocess.Popen(args, cwd=cwd, env=env)
    status = handle.wait()
    if status != 0:
        print(f"command {cmd} exited with failure status code")
        exit(status)

def with_exe_suffix(s: str) -> str:
    if os.name == "nt":
        return s + ".exe"
    else:
        return s


def cargo_build(bin: str, rustflags: [str] = None, features: [str] = None):
    args = ["cargo", "build", "--release", "--bin", bin]

    env = None
    if rustflags is not None:
        flags = ""
        for flag in rustflags:
            flags += f"--cfg {flag}"
        env = { "RUSTFLAGS": flags }
    
    if features is not None:
        args.append("--no-default-features")

        if len(features) > 0:
            featuresJoined = ",".join(features)
            args.append(f"--features={featuresJoined}")

    run_command(args, env=env)


class Args:
    tracy: bool
    embed_shaders: bool

    def parse() -> Args:
        parser = argparse.ArgumentParser(allow_abbrev=False)
        parser.add_argument("--tracy", help="Enable tracy profiler integration", action=argparse.BooleanOptionalAction, default=False)
        parser.add_argument("--embed-shaders", help="Embed shaders into the binary and disable runtime shader loading", action=argparse.BooleanOptionalAction, default=True)

        return parser.parse_args()

if __name__ == "__main__":
    main()
