#!/usr/bin/python3

import os
import shutil
import subprocess
import stat

# 755 permissions
RWX_PERMS = stat.S_IRUSR | stat.S_IWUSR | stat.S_IXUSR | stat.S_IRGRP | stat.S_IXGRP | stat.S_IROTH | stat.S_IXOTH

def main():
    cwd = os.getcwd()
    print(f"executing in {cwd}")

    env = { "RUSTFLAGS": "--cfg openfo4_embed_shaders" }

    run_command(["cargo", "build", "--release", "--bin", "openfo4_client"], env=env)
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

if __name__ == "__main__":
    main()
