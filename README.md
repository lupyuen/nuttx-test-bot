![Test Bot for Pull Requests ... Tested on Real Hardware (Apache NuttX RTOS / Oz64 SG2000 RISC-V SBC)](https://lupyuen.org/images/testbot-flow.jpg)

# Apache NuttX RTOS: Bot that will Build and Test a Pull Request

Read the article...

- ["Test Bot for Pull Requests ... Tested on Real Hardware (Apache NuttX RTOS / Oz64 SG2000 RISC-V SBC)"](https://lupyuen.org/articles/testbot.html)

- ["QEMU Test Bot for Pull Requests: Beware of Semihosting Breakout (Apache NuttX RTOS)"](https://lupyuen.org/articles/testbot2.html)

- ["PR Test Bot for PinePhone (Apache NuttX RTOS)"](https://lupyuen.org/articles/testbot3.html)

- ["Porting Apache NuttX RTOS to Avaota-A1 SBC (Allwinner A527 SoC)"](https://lupyuen.org/articles/avaota.html)


We might allow a [__PR Comment__](https://github.com/lupyuen/nuttx-test-bot/blob/main/src/main.rs) to trigger a Build + Test on QEMU. For example, this PR Comment...

```bash
@nuttxpr test rv-virt:knsh64
```

Will trigger our [__Test Bot__](https://github.com/lupyuen/nuttx-test-bot/blob/main/src/main.rs) to download the PR Code, and run Build + Test on QEMU RISC-V. Or on [__Real Hardware__](https://lupyuen.github.io/articles/sg2000a)...

```bash
@nuttxpr test milkv_duos:nsh
```

Here are commands for PR Test Bot:

```bash
## For Avaota-A1 Arm64 SBC (Allwinner A537)
@nuttxpr test avaota-a1:nsh

## For Milk-V Duo S 64-bit RISC-V SBC (Sophgo SG2000)
@nuttxpr test milkv_duos:nsh

## For QEMU Arm64 (Flat Build) and RISC-V 64-bit (Kernel Build)
@nuttxpr test qemu-armv8a:netnsh
@nuttxpr test rv-virt:knsh64
```

These commands will trigger an Email Alert to me. Please give me 12 Hours to review the PR [(making sure there isn't malicious code)](https://github.com/apache/nuttx/issues/15731#issuecomment-2628647886) before I start the PR Test Bot. The results shall be posted as a PR Comment.

# How To Run

See [run.sh](run.sh)...

```bash
#!/usr/bin/env bash
## Build and Test PRs for NuttX Kernel and Apps

set -e  ## Stop on error

## Install QEMU Emulators
sudo apt install \
  qemu-system-riscv64 \
  qemu-system-aarch64

## Set the GitHub Token. Should have permission to Post PR Comments.
## export GITHUB_TOKEN=...
. $HOME/github-token.sh

## Set the GitLab Token for creating snippets
## export GITLAB_TOKEN=...
. $HOME/gitlab-token.sh

set -x  ## Echo commands

## Enable Rust Logging
export RUST_LOG=info 
export RUST_BACKTRACE=1

for (( ; ; ))
do
  cargo run
  sleep 300
done
```

![Build + Test Bot is hosted on this hefty Ubuntu Xeon Workstation](https://lupyuen.org/images/ci4-thinkstation.jpg)

<span style="font-size:80%">

[_Build + Test Bot is hosted on this hefty Ubuntu Xeon Workstation_](https://qoto.org/@lupyuen/113517788288458811)

</span>
