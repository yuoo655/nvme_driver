
# pci设备
https://medium.com/@michael2012zhao_67085/understanding-pci-node-in-fdt-769a894a13cc


# BAR是pcie设备上的控制器提供给os的一组寄存器.  用来接收命令
bar

NVMe驱动解析-关键的BAR空间 

https://mp.weixin.qq.com/s/mCm7rDpprAY6M8bdFpxmJA

http://www.ssdfans.com/?p=8210

http://www.ssdfans.com/?p=8171

http://www.ssdfans.com/?p=8171

http://www.ssdfans.com/?p=8210


一个PCIe设备，可能有若干个内部空间（属性可能不一样，比如有些可预读，有些不可预读）需要映射到内存空间，设备出厂时，这些空间的大小和属性都写在Configuration BAR寄存器里面，然后上电后，
系统软件读取这些BAR，分别为其分配对应的系统内存空间，并把相应的内存基地址写回到BAR。（BAR的地址其实是PCI总线域的地址，CPU访问的是存储器域的地址，CPU访问PCIe设备时，需要把总线域地址转换成存储器域的地址。）


设备内存用page划分

Physical Region Page

# prp
用一个简单的例子窥探NVMe的PRP规则 

https://mp.weixin.qq.com/s/9oFnJ9JWmGIh-mgVz3jk4Q

http://www.ssdfans.com/?p=8173

http://www.ssdfans.com/?p=8141




# linux 块设备驱动
https://www.bilibili.com/read/cv17063262



# NVMe驱动解析-响应I/O请求
https://mp.weixin.qq.com/s?__biz=MzIyNDU0ODk4OA==&mid=2247483711&idx=1&sn=726890a3d3729d5b688a1f51a95900e5&
chksm=e80c002cdf7b893a6cce50fd5387d10e3ebdbf49804d89d37c79315b7e7d5279b6759d361ccf&scene=126&&sessionid=1662083002#rd

## Device-to-device memory-transfer offload with P2PDMA
https://lwn.net/Articles/767281/

PCI devices expose memory to the host system in form of memory regions defined by base address registers (BARs). 
Those are regions mapped into the host's physical memory space. 
All regions are mapped into the same address space, and PCI DMA operations can use those addresses directly.
It is thus possible for a driver to configure a PCI DMA operation to perform transfers between the memory zones of two devices while bypassing system memory completely. 

# linux地址空间  pcie dma
https://www.oreilly.com/library/view/linux-device-drivers/0596005903/ch15.html

NVMe驱动解析-DMA传输 https://mp.weixin.qq.com/s/iF6LHniCjYCZ1kAnw3x9cQ


Host如果想往SSD上写入用户数据，需要告诉SSD写入什么数据，

写入多少数据，以及数据源在内存中的什么位置，这些信息包含在Host向SSD发送的Write命令中。

每笔用户数据对应着一个叫做LBA（Logical Block Address）的东西，Write命令通过指定LBA来告诉SSD写入的是什么数据。

对NVMe/PCIe来说，SSD收到Write命令后，通过PCIe去Host的内存数据所在位置读取数据，然后把这些数据写入到闪存中，同时得到LBA与闪存位置的映射关系。





但是，还有一个问题，这个Admin Command是怎么传过去的呢？还是要看NVMe Spec。之前提到的NVMe的BAR空间中就有这么两个寄存器，它们用来存储Admin Queue 的 Command DMA基地址。
图片
如下，在创建Admin Queue的时候就向Controller写入DMA地址：



# Doorbellregister
SQ位于Host内存中，Host要发送命令时，先把准备好的命令放在SQ中，然后通知SSD来取；
CQ也是位于Host内存中，一个命令执行完成，成功或失败，SSD总会往CQ中写入命令完成状态。
DB又是干什么用的呢？Host发送命令时，不是直接往SSD中发送命令的，而是把命令准备好放在自己的内存中，
那怎么通知SSD来获取命令执行呢？
Host就是通过写SSD端的DB寄存器来告知SSD的

SQ = Submission Queue
CQ = Completion Queue
DB = Doorbell Register

第一步：Host写命令到SQ；

第二步：Host写DB，通知SSD取指；

第三步：SSD收到通知，于是从SQ中取指；

第四步：SSD执行指令；

第五步：指令执行完成，SSD往CQ中写指令执行结果；

第六步：然后SSD发起中断通知Host指令完成；

第七步：收到中断，Host处理CQ，查看指令完成状态；

第八步：Host处理完CQ中的指令执行结果，通过DB回复SSD：指令执行结果已处理，辛苦您了！



host往sq1中写入3个命令, sq1.tail=3, qs DBR = 3, 

执行完2个命令, cq DBR=2


db记录了sq 和 cq 的头和尾

ssd 控制器知道sq的head位置

host知道sq的tail位置

SSD往CQ中写入命令状态信息的同时，还把SQ Head DB的信息告知了Host

cq host 知道head 不知道tail
一开始cq中每条命令完成条目中的 p bit初始化为0, ssd在往cq中写入命令完成条目是p bit置为1, host在处理cq中的命令完成条目时, p bit置为0,
cq是在host的内存中, hist记住上次的tail, 检查p 得出新的tail




# nvme设备初始化

参考 <https://blog.csdn.net/yiyeguzhou100/article/details/105478124>

## 1. 创建admin queue

linux 5.19

```c
static int nvme_pci_configure_admin_queue(struct nvme_dev *dev)

```


u-boot

```c
static int nvme_configure_admin_queue(struct nvme_dev *dev)
{
}

struct nvme_bar {
	__u64 cap;	/* Controller Capabilities */
	__u32 vs;	/* Version */
	__u32 intms;	/* Interrupt Mask Set */
	__u32 intmc;	/* Interrupt Mask Clear */
	__u32 cc;	/* Controller Configuration */
	__u32 rsvd1;	/* Reserved */
	__u32 csts;	/* Controller Status */
	__u32 rsvd2;	/* Reserved */
	__u32 aqa;	/* Admin Queue Attributes */
	__u64 asq;	/* Admin SQ Base Address */
	__u64 acq;	/* Admin CQ Base Address */
};
```


# 更多参考
<https://blog.csdn.net/panzhenjie/article/details/51581063>

<https://nvmexpress.org/developers/nvme-specification/>


