1. 正确进入 U 态后，程序的特征还应有：使用 S 态特权指令，访问 S 态寄存器后会报错。 请同学们可以自行测试这些内容（运行 [三个 bad 测例 (ch2b_bad_*.rs)](https://github.com/LearningOS/rCore-Tutorial-Test-2024S/tree/master/src/bin) ）， 描述程序出错行为，同时注意注明你使用的 sbi 及其版本。

   > rustsbi版本：`[rustsbi] RustSBI version 0.3.0-alpha.2, adapting to RISC-V SBI v1.0.0`，`ch2b_bad_address.rs`程序发生页错误，访问非法地址 `0x0`，指令地址 `0x804003ac`；`ch2b_bad_instruction.rs`和 `ch2b_bad_register.rs`程序在U态下访问S态的指令和寄存器报错被内核kill掉
   >
2. 深入理解 [trap.S](https://github.com/LearningOS/rCore-Tutorial-Code-2024S/blob/ch3/os/src/trap/trap.S) 中两个函数 `<span class="pre">__alltraps</span>` 和 `<span class="pre">__restore</span>` 的作用，并回答如下问题

   1. L40：刚进入 `__restore` 时，`a0` 代表了什么值。请指出 `__restore` 的两种使用情景

      > 刚进 `__restore`时，`a0`在原ch2分支中为新建的 `TrapContext`用于指示返回用户态后执行起始地址，这也可以从后续的 `mv sp, a0`指令可知；但在ch3分支中移除了 `mv sp, a0`，在 `goto_restore`封装中并无无输入参数，`__swtch`之后 `sp`已经正确指向所需的Trap上下文地址。`__restore`在起始运行程序和在处理trap后返回U态时使用
      >
   2. L43-L48：这几行汇编代码特殊处理了哪些寄存器？这些寄存器的的值对于进入用户态有何意义？请分别解释。

      ```
      ld t0, 32*8(sp)
      ld t1, 33*8(sp)
      ld t2, 2*8(sp)
      csrw sstatus, t0
      csrw sepc, t1
      csrw sscratch, t2
      ```

      > 这几行汇编代码处理了CSR寄存器（`sstatus`、`sepc`、`sscratch`），这三个寄存器中存储了trap前的信息，若不先恢复这些寄存器的值，则无法恢复此前的三个临时寄存器（`t0`、`t1`、`t2`）
      >
   3. L50-L56：为何跳过了 `x2` 和 `x4`？

      ```
      ld x1, 1*8(sp)
      ld x3, 3*8(sp)
      .set n, 5
      .rep t27
          LOAD_GP %n
          .set n, n+1
      .endr
      ```

      > 因为 `x0`被硬编码为0，而 `x4`则不常用到，除非手动出于特殊用途使用，因此无需保存和恢复
      >
   4. L60：该指令之后，`sp` 和 `sscratch` 中的值分别有什么意义？

      ```
      csrrw sp, sscratch, sp
      ```

      > `csrrw`指令将 `sscratch`和 `sp`中的值交换，此前 `sscratch->user stack`，`sp->kernel stack`；结果 `sscratch->kernel stack`，`sp->user stack`
      >
   5. `__restore`：中发生状态切换在哪一条指令？为何该指令执行之后会进入用户态？

      > 状态切换发生在 `sret`指令，指令执行后CPU会将当前的特权级按照 `sstatus`的 `SPP`字段设置为U；之后跳转到 `sepc`寄存器指向的那条指令，然后继续执行。
      >
   6. L13：该指令之后，`sp`和 `sscratch`中的值分别有什么意义？

      ```
      csrrw sp, sscratch, sp
      ```

      交换 `sscratch`和 `sp`中的值，交换后 `sscratch->user stack`，`sp->kernel stack`
   7. 从U态进入S态是哪一条指令发生的？

      > ecall指令，在应用程序启动、发起系统调用、执行出错、执行结束时执行切换到S态进行处理
      >

---

1. 在完成本次实验的过程（含此前学习的过程）中，我曾分别与 **以下各位** 就（与本次实验相关的）以下方面做过交流，还在代码中对应的位置以注释形式记录了具体的交流对象及内容：

   > 无
   >
2. 此外，我也参考了 **以下资料** ，还在代码中对应的位置以注释形式记录了具体的参考来源及内容：

   > 无
   >
3. 我独立完成了本次实验除以上方面之外的所有工作，包括代码与文档。 我清楚地知道，从以上方面获得的信息在一定程度上降低了实验难度，可能会影响起评分。
4. 我从未使用过他人的代码，不管是原封不动地复制，还是经过了某些等价转换。 我未曾也不会向他人（含此后各届同学）复制或公开我的实验代码，我有义务妥善保管好它们。 我提交至本实验的评测系统的代码，均无意于破坏或妨碍任何计算机系统的正常运转。 我清楚地知道，以上情况均为本课程纪律所禁止，若违反，对应的实验成绩将按“-100”分计。
