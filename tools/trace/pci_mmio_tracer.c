// SPDX-License-Identifier: GPL-2.0
/*
 * RamenOS S11.1 PCI/MMIO trace scaffold for Linux Oracle capsules.
 *
 * This module does not pretend to hook arbitrary hardware accesses. It provides
 * a small, auditable trace buffer and an exported record helper that explicit
 * Oracle instrumentation can call from kprobes, wrappers, or a patched driver.
 * The host-side Foundry tooling converts the JSONL stream into
 * DriverProtocolTraceV0.
 */

#include <linux/debugfs.h>
#include <linux/ktime.h>
#include <linux/module.h>
#include <linux/mutex.h>
#include <linux/seq_file.h>
#include <linux/types.h>

#define RAMEN_TRACE_CAPACITY 4096

enum ramen_pci_mmio_event_kind {
	RAMEN_EVT_PCI_CONFIG_READ = 0,
	RAMEN_EVT_PCI_CONFIG_WRITE = 1,
	RAMEN_EVT_MMIO_READ = 2,
	RAMEN_EVT_MMIO_WRITE = 3,
	RAMEN_EVT_IRQ = 4,
	RAMEN_EVT_DMA_MAP = 5,
	RAMEN_EVT_DMA_UNMAP = 6,
};

struct ramen_pci_mmio_event {
	u64 seq;
	u64 timestamp_ns;
	u8 kind;
	u8 bar;
	u8 width;
	u8 reserved;
	u64 offset;
	u64 value;
};

static struct ramen_pci_mmio_event trace_events[RAMEN_TRACE_CAPACITY];
static DEFINE_MUTEX(trace_lock);
static u64 trace_seq;
static u32 trace_head;
static struct dentry *trace_dir;

static unsigned int target_vendor = 0x1af4;
module_param(target_vendor, uint, 0444);
MODULE_PARM_DESC(target_vendor, "PCI vendor id to document in capture metadata");

static unsigned int target_device = 0x1000;
module_param(target_device, uint, 0444);
MODULE_PARM_DESC(target_device, "PCI device id to document in capture metadata");

static char *target_bdf = "0000:00:03.0";
module_param(target_bdf, charp, 0444);
MODULE_PARM_DESC(target_bdf, "PCI BDF to document in capture metadata");

static const char *kind_name(u8 kind)
{
	switch (kind) {
	case RAMEN_EVT_PCI_CONFIG_READ:
		return "pci_config_read";
	case RAMEN_EVT_PCI_CONFIG_WRITE:
		return "pci_config_write";
	case RAMEN_EVT_MMIO_READ:
		return "mmio_read";
	case RAMEN_EVT_MMIO_WRITE:
		return "mmio_write";
	case RAMEN_EVT_IRQ:
		return "irq";
	case RAMEN_EVT_DMA_MAP:
		return "dma_map";
	case RAMEN_EVT_DMA_UNMAP:
		return "dma_unmap";
	default:
		return "unknown";
	}
}

void ramen_pci_mmio_trace_record(u8 kind, u8 bar, u64 offset, u8 width, u64 value)
{
	struct ramen_pci_mmio_event *event;

	mutex_lock(&trace_lock);
	event = &trace_events[trace_head % RAMEN_TRACE_CAPACITY];
	event->seq = ++trace_seq;
	event->timestamp_ns = ktime_get_ns();
	event->kind = kind;
	event->bar = bar;
	event->width = width;
	event->reserved = 0;
	event->offset = offset;
	event->value = value;
	trace_head++;
	mutex_unlock(&trace_lock);
}
EXPORT_SYMBOL_GPL(ramen_pci_mmio_trace_record);

static int events_show(struct seq_file *m, void *v)
{
	u32 count;
	u32 start;
	u32 i;

	mutex_lock(&trace_lock);
	count = trace_head > RAMEN_TRACE_CAPACITY ? RAMEN_TRACE_CAPACITY : trace_head;
	start = trace_head > RAMEN_TRACE_CAPACITY ? trace_head - RAMEN_TRACE_CAPACITY : 0;

	seq_printf(m,
		   "{\"metadata\":{\"oracle\":\"linux-virtio-net\","
		   "\"device_model\":\"virtio-net-pci\","
		   "\"pci_vendor_id\":%u,\"pci_device_id\":%u,"
		   "\"pci_bdf\":\"%s\",\"capture_tool\":\"pci_mmio_tracer\"}}\n",
		   target_vendor, target_device, target_bdf);

	for (i = 0; i < count; i++) {
		const struct ramen_pci_mmio_event *event =
			&trace_events[(start + i) % RAMEN_TRACE_CAPACITY];

		seq_printf(m,
			   "{\"seq\":%llu,\"timestamp_ns\":%llu,"
			   "\"kind\":\"%s\",\"bar\":%u,\"offset\":%llu,"
			   "\"width\":%u,\"value\":%llu,\"result\":\"ok\"}\n",
			   event->seq, event->timestamp_ns, kind_name(event->kind),
			   event->bar, event->offset, event->width, event->value);
	}
	mutex_unlock(&trace_lock);

	return 0;
}

static int events_open(struct inode *inode, struct file *file)
{
	return single_open(file, events_show, inode->i_private);
}

static const struct file_operations events_fops = {
	.owner = THIS_MODULE,
	.open = events_open,
	.read = seq_read,
	.llseek = seq_lseek,
	.release = single_release,
};

static int __init ramen_pci_mmio_tracer_init(void)
{
	trace_dir = debugfs_create_dir("ramen_pci_mmio_tracer", NULL);
	if (IS_ERR_OR_NULL(trace_dir))
		return -ENOMEM;

	debugfs_create_file("events", 0444, trace_dir, NULL, &events_fops);
	pr_info("ramen pci_mmio_tracer loaded target=%04x:%04x bdf=%s\n",
		target_vendor, target_device, target_bdf);
	return 0;
}

static void __exit ramen_pci_mmio_tracer_exit(void)
{
	debugfs_remove_recursive(trace_dir);
	pr_info("ramen pci_mmio_tracer unloaded\n");
}

module_init(ramen_pci_mmio_tracer_init);
module_exit(ramen_pci_mmio_tracer_exit);

MODULE_LICENSE("GPL");
MODULE_AUTHOR("RamenOS Foundry");
MODULE_DESCRIPTION("RamenOS S11 PCI/MMIO Oracle trace scaffold");
