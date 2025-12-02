<script lang="ts">
    import { client } from "@/lib/api";
    import { getAuthContext, getHackathonContext } from "@/lib/auth.svelte";
    import {
        Edit01Icon,
        LogOut01Icon,
        PlusIcon,
        SearchMdIcon,
        UserPlus01Icon,
        XCloseIcon,
    } from "@untitled-theme/icons-svelte";

    const auth = getAuthContext();
    const hackathon = getHackathonContext();

    // State
    let teams = $state<any[]>([]);
    let myTeam = $state<any | null>(null);
    let pendingInvites = $state<any[]>([]);
    let joinRequests = $state<any[]>([]);
    let searchQuery = $state("");
    let loading = $state(true);

    // Modals
    let showCreateModal = $state(false);
    let showViewModal = $state(false);
    let showJoinRequestModal = $state(false);
    let showInviteModal = $state(false);
    let showEditModal = $state(false);

    // Selected data
    let selectedTeam = $state<any | null>(null);
    let newTeamName = $state("");
    let newTeamDescription = $state("");
    let joinMessage = $state("");
    let inviteSearchQuery = $state("");
    let inviteSearchResults = $state<any[]>([]);
    let editTeamName = $state("");
    let editTeamDescription = $state("");
    let createInviteSearchQuery = $state("");
    let createInviteSearchResults = $state<any[]>([]);
    let selectedInviteMembers = $state<any[]>([]);
    let showCreateInviteDropdown = $state(false);

    // Active tab for my team view
    let activeTab = $state<"current" | "requests">("current");

    // Load data on mount
    $effect(() => {
        loadData();
    });

    async function loadData() {
        loading = true;
        await Promise.all([loadTeams(), loadMyTeam()]);
        loading = false;
    }

    async function loadTeams() {
        const { data } = await client.GET("/hackathons/{slug}/teams", {
            params: { path: { slug: hackathon.hackathonId! } },
        });
        if (data) {
            teams = data;
        }
    }

    async function loadMyTeam() {
        const { data, error, response } = await client.GET(
            "/hackathons/{slug}/teams/my-team",
            {
                params: { path: { slug: hackathon.hackathonId! } },
            },
        );
        console.log("loadMyTeam response:", {
            data,
            error,
            status: response?.status,
        });
        if (data) {
            myTeam = data.team;
            pendingInvites = data.pending_invites;

            // Load join requests if user has a team
            if (myTeam) {
                await loadJoinRequests();
            }
        }
    }

    async function loadJoinRequests() {
        if (!myTeam) return;
        const { data } = await client.GET(
            "/hackathons/{slug}/teams/{team_id}/requests",
            {
                params: {
                    path: { slug: hackathon.hackathonId!, team_id: myTeam.id },
                },
            },
        );
        if (data) {
            joinRequests = data;
        }
    }

    async function createTeam() {
        const { data, error } = await client.POST("/hackathons/{slug}/teams", {
            params: { path: { slug: hackathon.hackathonId! } },
            body: {
                name: newTeamName,
                description: newTeamDescription || undefined,
            },
        });

        if (data) {
            // Invite selected members after team creation
            for (const member of selectedInviteMembers) {
                await client.POST("/hackathons/{slug}/teams/{team_id}/invite", {
                    params: {
                        path: {
                            slug: hackathon.hackathonId!,
                            team_id: data.id,
                        },
                    },
                    body: { user_id: member.user_id },
                });
            }

            showCreateModal = false;
            newTeamName = "";
            newTeamDescription = "";
            selectedInviteMembers = [];
            createInviteSearchQuery = "";
            createInviteSearchResults = [];
            await loadData();
        }
    }

    async function viewTeam(team: any) {
        console.log("viewTeam called with:", team);
        console.log("hackathonId:", hackathon.hackathonId);
        const { data, error, response } = await client.GET(
            "/hackathons/{slug}/teams/{team_id}",
            {
                params: {
                    path: { slug: hackathon.hackathonId!, team_id: team.id },
                },
            },
        );
        console.log("API response:", {
            data,
            error,
            status: response?.status,
            ok: response?.ok,
        });
        if (data) {
            selectedTeam = data;
            showViewModal = true;
        }
    }

    async function requestToJoin() {
        if (!selectedTeam) return;

        await client.POST("/hackathons/{slug}/teams/{team_id}/request", {
            params: {
                path: {
                    slug: hackathon.hackathonId!,
                    team_id: selectedTeam.id,
                },
            },
            body: { message: joinMessage || undefined },
        });

        showJoinRequestModal = false;
        joinMessage = "";
        showViewModal = false;
    }

    async function respondToJoinRequest(requestId: number, accept: boolean) {
        if (!myTeam) return;

        await client.POST(
            "/hackathons/{slug}/teams/{team_id}/requests/{request_id}",
            {
                params: {
                    path: {
                        slug: hackathon.hackathonId!,
                        team_id: myTeam.id,
                        request_id: requestId,
                    },
                },
                body: { accept },
            },
        );

        await loadData();
    }

    async function respondToInvite(inviteId: number, accept: boolean) {
        await client.POST("/hackathons/{slug}/teams/invites/{invite_id}", {
            params: {
                path: { slug: hackathon.hackathonId!, invite_id: inviteId },
            },
            body: { accept },
        });

        await loadData();
    }

    async function leaveTeam() {
        if (!myTeam) return;

        // If user is leader and there are other members, show a specific message
        if (myTeam.is_leader && myTeam.member_count > 1) {
            alert(
                "You cannot leave the team while you are the leader. Please transfer leadership to another member first, or remove all other members before leaving.",
            );
            return;
        }

        const confirmMessage =
            myTeam.member_count === 1
                ? "Are you sure you want to leave this team? Since you're the only member, the team will be deleted."
                : "Are you sure you want to leave this team?";

        if (confirm(confirmMessage)) {
            const { error, response } = await client.POST(
                "/hackathons/{slug}/teams/{team_id}/leave",
                {
                    params: {
                        path: {
                            slug: hackathon.hackathonId!,
                            team_id: myTeam.id,
                        },
                    },
                },
            );

            if (error || !response.ok) {
                alert(
                    "Failed to leave team. If you are the leader, you must transfer leadership first.",
                );
                return;
            }

            await loadData();
        }
    }

    async function searchParticipants() {
        if (!inviteSearchQuery) {
            inviteSearchResults = [];
            return;
        }

        const { data } = await client.GET(
            "/hackathons/{slug}/teams/search-participants",
            {
                params: {
                    path: { slug: hackathon.hackathonId! },
                    query: { q: inviteSearchQuery },
                },
            },
        );

        if (data) {
            inviteSearchResults = data;
        }
    }

    async function searchParticipantsForCreate() {
        if (!createInviteSearchQuery) {
            createInviteSearchResults = [];
            return;
        }

        const { data } = await client.GET(
            "/hackathons/{slug}/teams/search-participants",
            {
                params: {
                    path: { slug: hackathon.hackathonId! },
                    query: { q: createInviteSearchQuery },
                },
            },
        );

        if (data) {
            // Filter out already selected members
            createInviteSearchResults = data.filter(
                (user: any) =>
                    !selectedInviteMembers.some(
                        (m) => m.user_id === user.user_id,
                    ),
            );
        }
    }

    function addInviteMember(user: any) {
        selectedInviteMembers = [...selectedInviteMembers, user];
        createInviteSearchQuery = "";
        createInviteSearchResults = [];
        showCreateInviteDropdown = false;
    }

    function removeInviteMember(userId: number) {
        selectedInviteMembers = selectedInviteMembers.filter(
            (m) => m.user_id !== userId,
        );
    }

    async function inviteMember(userId: number) {
        if (!myTeam) return;

        await client.POST("/hackathons/{slug}/teams/{team_id}/invite", {
            params: {
                path: { slug: hackathon.hackathonId!, team_id: myTeam.id },
            },
            body: { user_id: userId },
        });

        inviteSearchQuery = "";
        inviteSearchResults = [];
        showInviteModal = false;
    }

    async function updateTeam() {
        if (!myTeam) return;

        await client.PUT("/hackathons/{slug}/teams/{team_id}", {
            params: {
                path: { slug: hackathon.hackathonId!, team_id: myTeam.id },
            },
            body: {
                name: editTeamName,
                description: editTeamDescription || undefined,
            },
        });

        showEditModal = false;
        await loadData();
    }

    function openEditModal() {
        editTeamName = myTeam?.name || "";
        editTeamDescription = myTeam?.description || "";
        showEditModal = true;
    }

    // Filter teams by search
    let filteredTeams = $derived(
        teams.filter(
            (team) =>
                team.name.toLowerCase().includes(searchQuery.toLowerCase()) ||
                (team.description &&
                    team.description
                        .toLowerCase()
                        .includes(searchQuery.toLowerCase())),
        ),
    );

    // Get random color for avatar
    function getAvatarColor(index: number) {
        const colors = [
            "bg-orange-400",
            "bg-purple-500",
            "bg-pink-400",
            "bg-blue-400",
            "bg-green-400",
            "bg-yellow-400",
        ];
        return colors[index % colors.length];
    }
</script>

<div class="flex flex-col gap-6 p-6">
    <!-- My Team Section (if user has a team) -->
    {#if myTeam}
        <section>
            <div class="flex items-center justify-between mb-4">
                <h1 class="text-3xl font-bold">My Team</h1>
                <button
                    onclick={leaveTeam}
                    class="flex items-center gap-2 px-4 py-2 border border-gray-300 rounded-full hover:bg-gray-100"
                >
                    <LogOut01Icon class="size-4" />
                    Leave Team
                </button>
            </div>

            <!-- Team Info Card -->
            <div class="bg-white rounded-2xl p-6 shadow-sm">
                <div class="flex justify-between items-start">
                    <div>
                        <p class="text-sm text-gray-400">Team Name</p>
                        <h2 class="text-xl font-semibold">{myTeam.name}</h2>
                    </div>
                    {#if myTeam.is_leader}
                        <button
                            onclick={openEditModal}
                            class="flex items-center gap-2 px-3 py-1.5 border border-gray-300 rounded-full hover:bg-gray-100 text-sm"
                        >
                            <Edit01Icon class="size-4" />
                            Edit
                        </button>
                    {/if}
                </div>

                <div class="mt-6">
                    <p class="text-sm text-gray-400">Description</p>
                    <p class="text-gray-700 mt-1">
                        {myTeam.description || "No description provided."}
                    </p>
                </div>
            </div>

            <!-- Members Card -->
            <div class="bg-white rounded-2xl p-6 shadow-sm mt-4">
                <div class="flex justify-between items-center mb-4">
                    <h3 class="text-lg font-semibold">Members</h3>
                    {#if myTeam.is_leader && myTeam.member_count < myTeam.max_members}
                        <button
                            onclick={() => (showInviteModal = true)}
                            class="flex items-center gap-2 px-3 py-1.5 border border-gray-300 rounded-full hover:bg-gray-100 text-sm"
                        >
                            <UserPlus01Icon class="size-4" />
                            Invite Members
                        </button>
                    {/if}
                </div>

                <!-- Tabs -->
                <div class="flex gap-1 mb-4">
                    <button
                        onclick={() => (activeTab = "current")}
                        class="px-4 py-1.5 rounded-full text-sm {activeTab ===
                        'current'
                            ? 'bg-blue-100 text-blue-700'
                            : 'hover:bg-gray-100'}"
                    >
                        Current
                    </button>
                    <button
                        onclick={() => (activeTab = "requests")}
                        class="px-4 py-1.5 rounded-full text-sm {activeTab ===
                        'requests'
                            ? 'bg-blue-100 text-blue-700'
                            : 'hover:bg-gray-100'}"
                    >
                        Requests
                        {#if joinRequests.length > 0}
                            <span
                                class="ml-1 bg-red-500 text-white rounded-full px-2 py-0.5 text-xs"
                                >{joinRequests.length}</span
                            >
                        {/if}
                    </button>
                </div>

                {#if activeTab === "current"}
                    <div class="space-y-3">
                        {#each myTeam.members as member, i}
                            <div class="flex items-center gap-3">
                                <div
                                    class="w-8 h-8 rounded-full {getAvatarColor(
                                        i,
                                    )}"
                                ></div>
                                <span class="font-medium text-sm">
                                    {member.name || member.email}
                                </span>
                            </div>
                        {/each}
                    </div>
                {:else}
                    <div class="space-y-3">
                        {#if joinRequests.length === 0}
                            <p class="text-gray-500 text-center py-4">
                                No pending requests
                            </p>
                        {:else}
                            {#each joinRequests as request, i}
                                <div
                                    class="flex items-center justify-between p-3 bg-gray-50 rounded-xl"
                                >
                                    <div class="flex items-center gap-3">
                                        <div
                                            class="w-8 h-8 rounded-full {getAvatarColor(
                                                i,
                                            )}"
                                        ></div>
                                        <div>
                                            <p class="font-medium text-sm">
                                                {request.user_name ||
                                                    request.user_email}
                                            </p>
                                            {#if request.message}
                                                <p
                                                    class="text-xs text-gray-500"
                                                >
                                                    {request.message}
                                                </p>
                                            {/if}
                                        </div>
                                    </div>
                                    {#if myTeam.is_leader}
                                        <div class="flex gap-2">
                                            <button
                                                onclick={() =>
                                                    respondToJoinRequest(
                                                        request.id,
                                                        false,
                                                    )}
                                                class="px-3 py-1.5 border border-gray-300 rounded-full hover:bg-gray-100 text-sm"
                                            >
                                                Reject
                                            </button>
                                            <button
                                                onclick={() =>
                                                    respondToJoinRequest(
                                                        request.id,
                                                        true,
                                                    )}
                                                class="px-3 py-1.5 bg-gray-900 text-white rounded-full hover:bg-gray-800 text-sm"
                                            >
                                                Accept
                                            </button>
                                        </div>
                                    {/if}
                                </div>
                            {/each}
                        {/if}
                    </div>
                {/if}
            </div>
        </section>
    {/if}

    <!-- Pending Invites -->
    {#if pendingInvites.length > 0}
        <section>
            <h2 class="text-xl font-bold mb-4">Pending Invites</h2>
            <div class="space-y-3">
                {#each pendingInvites as invite}
                    <div
                        class="flex items-center justify-between p-4 bg-white rounded-xl shadow-sm"
                    >
                        <div>
                            <p class="font-medium">{invite.team_name}</p>
                            <p class="text-sm text-gray-500">
                                Invited by {invite.invited_by_name}
                            </p>
                        </div>
                        <div class="flex gap-2">
                            <button
                                onclick={() =>
                                    respondToInvite(invite.id, false)}
                                class="px-3 py-1.5 border border-gray-300 rounded-full hover:bg-gray-100"
                            >
                                Decline
                            </button>
                            <button
                                onclick={() => respondToInvite(invite.id, true)}
                                class="px-3 py-1.5 bg-gray-900 text-white rounded-full hover:bg-gray-800"
                            >
                                Accept
                            </button>
                        </div>
                    </div>
                {/each}
            </div>
        </section>
    {/if}

    <!-- All Teams Section -->
    <section>
        <h2 class="text-3xl font-bold mb-4">All Teams</h2>

        <!-- Search and Create -->
        <div class="flex items-center justify-between gap-4 mb-4">
            <div
                class="flex items-center gap-2 px-4 py-2 bg-white rounded-full max-w-xs"
            >
                <SearchMdIcon class="size-5 text-gray-400" />
                <input
                    type="text"
                    placeholder="Search teams"
                    class="flex-1 outline-none bg-transparent text-sm"
                    bind:value={searchQuery}
                />
            </div>

            {#if !myTeam}
                <button
                    onclick={() => (showCreateModal = true)}
                    class="flex items-center gap-2 px-4 py-2 bg-gray-900 text-white rounded-full hover:bg-gray-800"
                >
                    <PlusIcon class="size-5" />
                    Create New Team
                </button>
            {/if}
        </div>

        <!-- Teams List -->
        {#if loading}
            <div class="text-center py-8">
                <p class="text-gray-500">Loading teams...</p>
            </div>
        {:else if filteredTeams.length === 0}
            <div class="text-center py-8">
                <p class="text-gray-500">No teams found</p>
            </div>
        {:else}
            <div
                class="bg-white rounded-2xl shadow-sm divide-y divide-gray-100"
            >
                {#each filteredTeams as team}
                    <div class="flex items-center justify-between px-6 py-4">
                        <div class="flex-1 min-w-0 mr-8">
                            <h3 class="font-semibold">{team.name}</h3>
                            {#if team.description}
                                <p class="text-sm text-gray-500 truncate">
                                    {team.description}
                                </p>
                            {/if}
                        </div>
                        <div class="flex items-center gap-8">
                            <span
                                class="text-sm text-gray-500 whitespace-nowrap"
                            >
                                {team.member_count}/{team.max_members} Members
                            </span>
                            <button
                                onclick={() => viewTeam(team)}
                                class="px-5 py-2 bg-gray-900 text-white rounded-full hover:bg-gray-800 text-sm font-medium"
                            >
                                View
                            </button>
                        </div>
                    </div>
                {/each}
            </div>
        {/if}
    </section>
</div>

<!-- Create Team Modal -->
{#if showCreateModal}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        onclick={() => {
            showCreateModal = false;
            showCreateInviteDropdown = false;
        }}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="bg-white rounded-2xl p-6 w-full max-w-md"
            role="document"
            onclick={(e) => e.stopPropagation()}
        >
            <h2 class="text-xl font-bold mb-4">Create new team</h2>

            <div class="space-y-4">
                <div>
                    <label
                        for="create-team-name"
                        class="block text-sm font-medium mb-1">Team Name</label
                    >
                    <input
                        id="create-team-name"
                        type="text"
                        placeholder="Enter"
                        class="w-full px-4 py-2 border border-gray-300 rounded-xl outline-none focus:ring-2 focus:ring-blue-300"
                        bind:value={newTeamName}
                    />
                </div>

                <div>
                    <label
                        for="create-team-description"
                        class="block text-sm font-medium mb-1"
                        >Team Description</label
                    >
                    <textarea
                        id="create-team-description"
                        placeholder="Add a description about your team"
                        class="w-full px-4 py-2 border border-gray-300 rounded-xl outline-none focus:ring-2 focus:ring-blue-300 resize-none"
                        rows="3"
                        bind:value={newTeamDescription}
                    ></textarea>
                </div>

                <div class="relative">
                    <label
                        for="create-invite-members"
                        class="block text-sm font-medium mb-1"
                        >Invite Members</label
                    >

                    <!-- Selected members tags -->
                    {#if selectedInviteMembers.length > 0}
                        <div class="flex flex-wrap gap-2 mb-2">
                            {#each selectedInviteMembers as member}
                                <span
                                    class="inline-flex items-center gap-1 px-3 py-1 bg-gray-100 rounded-full text-sm"
                                >
                                    {member.name || member.email}
                                    <button
                                        type="button"
                                        onclick={() =>
                                            removeInviteMember(member.user_id)}
                                        class="hover:bg-gray-200 rounded-full p-0.5"
                                    >
                                        <XCloseIcon class="size-3" />
                                    </button>
                                </span>
                            {/each}
                        </div>
                    {/if}

                    <!-- Dropdown trigger -->
                    <button
                        type="button"
                        id="create-invite-members"
                        class="w-full px-4 py-2 border border-gray-300 rounded-xl text-left flex items-center justify-between hover:border-gray-400"
                        onclick={() =>
                            (showCreateInviteDropdown =
                                !showCreateInviteDropdown)}
                    >
                        <span class="text-gray-500">Select</span>
                        <svg
                            class="size-5 text-gray-400"
                            fill="none"
                            stroke="currentColor"
                            viewBox="0 0 24 24"
                        >
                            <path
                                stroke-linecap="round"
                                stroke-linejoin="round"
                                stroke-width="2"
                                d="M19 9l-7 7-7-7"
                            />
                        </svg>
                    </button>

                    <!-- Dropdown content -->
                    {#if showCreateInviteDropdown}
                        <div
                            class="absolute z-10 w-full mt-1 bg-white border border-gray-300 rounded-xl shadow-lg"
                        >
                            <div class="p-2">
                                <input
                                    type="text"
                                    placeholder="Search participants..."
                                    class="w-full px-3 py-2 border border-gray-200 rounded-lg outline-none focus:ring-2 focus:ring-blue-300"
                                    bind:value={createInviteSearchQuery}
                                    oninput={searchParticipantsForCreate}
                                />
                            </div>
                            <div class="max-h-48 overflow-y-auto">
                                {#each createInviteSearchResults as user, i}
                                    <button
                                        type="button"
                                        class="w-full flex items-center gap-3 px-4 py-2 hover:bg-gray-50 text-left"
                                        onclick={() => addInviteMember(user)}
                                    >
                                        <div
                                            class="w-8 h-8 rounded-full {getAvatarColor(
                                                i,
                                            )}"
                                        ></div>
                                        <div>
                                            <p class="font-medium text-sm">
                                                {user.name || user.email}
                                            </p>
                                            <p class="text-xs text-gray-500">
                                                {user.email}
                                            </p>
                                        </div>
                                    </button>
                                {/each}
                                {#if createInviteSearchQuery && createInviteSearchResults.length === 0}
                                    <p
                                        class="text-center text-gray-500 py-3 text-sm"
                                    >
                                        No participants found
                                    </p>
                                {/if}
                                {#if !createInviteSearchQuery}
                                    <p
                                        class="text-center text-gray-400 py-3 text-sm"
                                    >
                                        Type to search participants
                                    </p>
                                {/if}
                            </div>
                        </div>
                    {/if}
                </div>
            </div>

            <div class="flex justify-end gap-3 mt-6">
                <button
                    onclick={() => {
                        showCreateModal = false;
                        showCreateInviteDropdown = false;
                        selectedInviteMembers = [];
                        createInviteSearchQuery = "";
                        createInviteSearchResults = [];
                    }}
                    class="px-4 py-2 hover:bg-gray-100 rounded-full"
                >
                    Cancel
                </button>
                <button
                    onclick={createTeam}
                    disabled={!newTeamName}
                    class="px-4 py-2 bg-gray-900 text-white rounded-full hover:bg-gray-800 disabled:opacity-50"
                >
                    Create
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- View Team Modal -->
{#if showViewModal && selectedTeam}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        onclick={() => (showViewModal = false)}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="bg-white rounded-2xl p-6 w-full max-w-md"
            role="document"
            onclick={(e) => e.stopPropagation()}
        >
            <div class="flex justify-end mb-2">
                <button onclick={() => (showViewModal = false)}>
                    <XCloseIcon class="size-6" />
                </button>
            </div>

            <div class="mb-4">
                <p class="text-sm text-gray-500">Team Name</p>
                <h2 class="text-2xl font-bold">{selectedTeam.name}</h2>
            </div>

            <div class="mb-6">
                <p class="text-sm text-gray-500 mb-1">Description</p>
                <p class="text-gray-700">
                    {selectedTeam.description || "No description provided."}
                </p>
            </div>

            <div class="mb-6">
                <p class="text-sm font-medium mb-3">Members</p>
                <div class="space-y-3">
                    {#each selectedTeam.members as member, i}
                        <div class="flex items-center gap-3">
                            <div
                                class="w-10 h-10 rounded-full {getAvatarColor(
                                    i,
                                )}"
                            ></div>
                            <span class="font-medium"
                                >{member.name || member.email}</span
                            >
                        </div>
                    {/each}
                </div>
            </div>

            {#if !myTeam}
                {#if selectedTeam.member_count < selectedTeam.max_members}
                    <div class="flex justify-end">
                        <button
                            onclick={() => (showJoinRequestModal = true)}
                            class="px-6 py-2 bg-gray-900 text-white rounded-full hover:bg-gray-800"
                        >
                            Join
                        </button>
                    </div>
                {:else}
                    <div class="flex justify-end">
                        <button
                            disabled
                            class="px-6 py-2 bg-gray-300 text-gray-500 rounded-full cursor-not-allowed"
                        >
                            Team is Full
                        </button>
                    </div>
                {/if}
            {/if}
        </div>
    </div>
{/if}

<!-- Join Request Modal -->
{#if showJoinRequestModal && selectedTeam}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        onclick={() => (showJoinRequestModal = false)}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="bg-white rounded-2xl p-6 w-full max-w-md"
            role="document"
            onclick={(e) => e.stopPropagation()}
        >
            <div class="flex items-start justify-between mb-4">
                <h2 class="text-xl font-bold">Request to join</h2>
                <div
                    class="w-12 h-12 rounded-full bg-green-500 flex items-center justify-center text-white font-medium"
                >
                    {(
                        auth.user?.first_name?.[0] ||
                        auth.user?.email?.[0] ||
                        "?"
                    ).toUpperCase()}
                </div>
            </div>

            <div>
                <label
                    for="join-message"
                    class="block text-sm text-gray-500 mb-2"
                    >Message (optional)</label
                >
                <textarea
                    id="join-message"
                    placeholder="Add a message"
                    class="w-full px-4 py-3 bg-gray-100 rounded-xl outline-none focus:ring-2 focus:ring-blue-300 resize-none"
                    rows="3"
                    bind:value={joinMessage}
                ></textarea>
            </div>

            <div class="flex justify-end gap-3 mt-6">
                <button
                    onclick={() => (showJoinRequestModal = false)}
                    class="px-4 py-2 hover:bg-gray-100 rounded-full"
                >
                    Cancel
                </button>
                <button
                    onclick={requestToJoin}
                    class="px-4 py-2 bg-gray-900 text-white rounded-full hover:bg-gray-800"
                >
                    Send
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- Invite Members Modal -->
{#if showInviteModal}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        onclick={() => (showInviteModal = false)}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="bg-white rounded-2xl p-6 w-full max-w-md"
            role="document"
            onclick={(e) => e.stopPropagation()}
        >
            <h2 class="text-xl font-bold mb-4">Invite Members</h2>

            <div
                class="flex items-center gap-2 px-4 py-2 bg-gray-100 rounded-xl mb-4"
            >
                <SearchMdIcon class="size-5 text-gray-400" />
                <input
                    type="text"
                    placeholder="Search participants..."
                    class="flex-1 outline-none bg-transparent"
                    bind:value={inviteSearchQuery}
                    oninput={searchParticipants}
                />
            </div>

            <div class="max-h-64 overflow-y-auto space-y-2">
                {#each inviteSearchResults as user, i}
                    <div
                        class="flex items-center justify-between p-3 hover:bg-gray-50 rounded-xl"
                    >
                        <div class="flex items-center gap-3">
                            <div
                                class="w-8 h-8 rounded-full {getAvatarColor(i)}"
                            ></div>
                            <div>
                                <p class="font-medium">
                                    {user.name || user.email}
                                </p>
                                <p class="text-sm text-gray-500">
                                    {user.email}
                                </p>
                            </div>
                        </div>
                        <button
                            onclick={() => inviteMember(user.user_id)}
                            class="px-3 py-1 bg-gray-900 text-white rounded-full text-sm hover:bg-gray-800"
                        >
                            Invite
                        </button>
                    </div>
                {/each}

                {#if inviteSearchQuery && inviteSearchResults.length === 0}
                    <p class="text-center text-gray-500 py-4">
                        No participants found
                    </p>
                {/if}
            </div>

            <div class="flex justify-end mt-6">
                <button
                    onclick={() => (showInviteModal = false)}
                    class="px-4 py-2 hover:bg-gray-100 rounded-full"
                >
                    Close
                </button>
            </div>
        </div>
    </div>
{/if}

<!-- Edit Team Modal -->
{#if showEditModal}
    <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions a11y_interactive_supports_focus -->
    <div
        class="fixed inset-0 bg-black/50 flex items-center justify-center z-50"
        role="dialog"
        aria-modal="true"
        onclick={() => (showEditModal = false)}
    >
        <!-- svelte-ignore a11y_click_events_have_key_events a11y_no_static_element_interactions -->
        <div
            class="bg-white rounded-2xl p-6 w-full max-w-md"
            role="document"
            onclick={(e) => e.stopPropagation()}
        >
            <h2 class="text-xl font-bold mb-4">Edit Team</h2>

            <div class="space-y-4">
                <div>
                    <label
                        for="edit-team-name"
                        class="block text-sm font-medium mb-1">Team Name</label
                    >
                    <input
                        id="edit-team-name"
                        type="text"
                        class="w-full px-4 py-2 border border-gray-300 rounded-xl outline-none focus:ring-2 focus:ring-blue-300"
                        bind:value={editTeamName}
                    />
                </div>

                <div>
                    <label
                        for="edit-team-description"
                        class="block text-sm font-medium mb-1"
                        >Team Description</label
                    >
                    <textarea
                        id="edit-team-description"
                        class="w-full px-4 py-2 border border-gray-300 rounded-xl outline-none focus:ring-2 focus:ring-blue-300 resize-none"
                        rows="3"
                        bind:value={editTeamDescription}
                    ></textarea>
                </div>
            </div>

            <div class="flex justify-end gap-3 mt-6">
                <button
                    onclick={() => (showEditModal = false)}
                    class="px-4 py-2 hover:bg-gray-100 rounded-full"
                >
                    Cancel
                </button>
                <button
                    onclick={updateTeam}
                    disabled={!editTeamName}
                    class="px-4 py-2 bg-gray-900 text-white rounded-full hover:bg-gray-800 disabled:opacity-50"
                >
                    Save
                </button>
            </div>
        </div>
    </div>
{/if}
