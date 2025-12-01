<script lang="ts">
    import { client } from "@/lib/api";
    import { getHackathonContext } from "@/lib/auth.svelte";
    import {
        File06Icon,
        Upload01Icon,
        XCloseIcon,
    } from "@untitled-theme/icons-svelte";

    interface Props {
        label: string;
        description?: string | null;
        required?: boolean;
        disabled?: boolean;
        value?: string;
        onInput?: (value: string) => void;
        accept?: string;
        maxSizeMB?: number;
        fieldId?: string;
    }

    let {
        label,
        description = null,
        required = false,
        disabled = false,
        value = "",
        onInput = () => {},
        accept = ".pdf",
        maxSizeMB = 5,
        fieldId = "file",
    }: Props = $props();

    const hackathon = getHackathonContext();

    let uploading = $state(false);
    let uploadError = $state<string | null>(null);
    let fileName = $state<string | null>(null);
    let dragOver = $state(false);

    // Extract filename from value if it exists
    $effect(() => {
        if (value) {
            // Value is stored as "filename|url" format
            const parts = value.split("|");
            if (parts.length >= 1) {
                fileName = parts[0];
            }
        } else {
            fileName = null;
        }
    });

    async function handleFileSelect(file: File | null) {
        if (!file || disabled) return;

        uploadError = null;

        // Validate file type
        const allowedExtensions = accept
            .split(",")
            .map((ext) => ext.trim().toLowerCase());
        const fileExtension = "." + file.name.split(".").pop()?.toLowerCase();
        if (!allowedExtensions.includes(fileExtension)) {
            uploadError = `Invalid file type. Allowed: ${accept}`;
            return;
        }

        // Validate file size
        const maxBytes = maxSizeMB * 1024 * 1024;
        if (file.size > maxBytes) {
            uploadError = `File too large. Maximum size: ${maxSizeMB}MB`;
            return;
        }

        uploading = true;

        try {
            // Get presigned upload URL from backend
            const { data: presignedData, error: presignedError } =
                await client.POST("/hackathons/{slug}/application/upload-url", {
                    params: { path: { slug: hackathon.hackathonId! } },
                    body: {
                        field_id: fieldId,
                        file_name: file.name,
                        content_type: file.type || "application/pdf",
                    },
                });

            if (presignedError || !presignedData) {
                throw new Error("Failed to get upload URL");
            }

            // Upload file directly to S3/MinIO
            const uploadResponse = await fetch(presignedData.upload_url, {
                method: "PUT",
                body: file,
                headers: {
                    "Content-Type": file.type || "application/pdf",
                },
            });

            if (!uploadResponse.ok) {
                throw new Error("Failed to upload file");
            }

            // Store filename and key in value
            onInput(`${file.name}|${presignedData.file_key}`);
        } catch (err) {
            uploadError = err instanceof Error ? err.message : "Upload failed";
        } finally {
            uploading = false;
        }
    }

    function handleInputChange(event: Event) {
        const input = event.target as HTMLInputElement;
        const file = input.files?.[0] || null;
        handleFileSelect(file);
    }

    function handleDrop(event: DragEvent) {
        event.preventDefault();
        dragOver = false;
        const file = event.dataTransfer?.files?.[0] || null;
        handleFileSelect(file);
    }

    function handleDragOver(event: DragEvent) {
        event.preventDefault();
        if (!disabled) {
            dragOver = true;
        }
    }

    function handleDragLeave() {
        dragOver = false;
    }

    function removeFile() {
        if (disabled) return;
        onInput("");
        fileName = null;
    }
</script>

<div class="flex flex-col gap-2">
    <span class="text-lg text-gray-800">
        {label}
        {#if required}
            <span class="text-error">*</span>
        {/if}
    </span>
    {#if description}
        <p class="text-sm text-muted-foreground">{description}</p>
    {/if}

    {#if value && fileName}
        <!-- File uploaded state -->
        <div
            class="flex items-center gap-3 p-4 rounded-xl bg-green-50 border border-green-200"
        >
            <File06Icon class="size-6 text-green-600 flex-shrink-0" />
            <span class="flex-1 text-gray-700 truncate">{fileName}</span>
            {#if !disabled}
                <button
                    type="button"
                    onclick={removeFile}
                    class="p-1 rounded-lg hover:bg-green-100 text-gray-500 hover:text-gray-700"
                    aria-label="Remove file"
                >
                    <XCloseIcon class="size-5" />
                </button>
            {/if}
        </div>
    {:else}
        <!-- Upload area -->
        <label
            class="flex flex-col items-center justify-center gap-3 p-6 rounded-xl border-2 border-dashed transition-colors cursor-pointer
                {disabled
                ? 'bg-gray-100 border-gray-200 cursor-not-allowed'
                : dragOver
                  ? 'bg-blue-50 border-blue-400'
                  : 'bg-slate-50 border-gray-300 hover:border-blue-400 hover:bg-blue-50'}"
            ondrop={handleDrop}
            ondragover={handleDragOver}
            ondragleave={handleDragLeave}
        >
            {#if uploading}
                <div
                    class="w-8 h-8 border-3 border-blue-500 border-t-transparent rounded-full animate-spin"
                ></div>
                <span class="text-sm text-gray-600">Uploading...</span>
            {:else}
                <Upload01Icon class="size-8 text-gray-400" />
                <div class="text-center">
                    <span class="text-sm text-gray-600">
                        <span class="text-blue-600 font-medium"
                            >Click to upload</span
                        >
                        or drag and drop
                    </span>
                    <p class="text-xs text-gray-400 mt-1">
                        {accept.toUpperCase().replace(/\./g, "")} (max {maxSizeMB}MB)
                    </p>
                </div>
            {/if}
            <input
                type="file"
                {accept}
                {disabled}
                class="hidden"
                onchange={handleInputChange}
            />
        </label>
    {/if}

    {#if uploadError}
        <p class="text-sm text-red-500">{uploadError}</p>
    {/if}
</div>
