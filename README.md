# Apache NuttX RTOS: Bot that will Build and Test a Pull Request

We might allow a __PR Comment__ to trigger a Build + Test on QEMU. For example, this PR Comment...

```bash
@nuttxpr test rv-virt:knsh64
```

Will trigger our __Test Bot__ to download the PR Code, and run Build + Test on QEMU RISC-V. Or on __Real Hardware__...

```bash
@nuttxpr test milkv_duos:nsh
```

Super helpful for __Testing Pull Requests__ before Merging. But might have [__Security Implications__](https://github.com/apache/nuttx/issues/15731#issuecomment-2628647886) 🤔

![Daily Test + Rewind is hosted on this hefty Ubuntu Xeon Workstation](https://lupyuen.org/images/ci4-thinkstation.jpg)
