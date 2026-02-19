const searchInput = document.getElementById("search");
const resultsList = document.getElementById("results");
const entityIdInput = document.getElementById("entity_id");
const submitBtn = document.getElementById("submit-btn");

let debounceTimer = null;

searchInput.addEventListener("input", () => {
    clearTimeout(debounceTimer);
    debounceTimer = setTimeout(() => {
        const query = searchInput.value.trim();
        if (query.length < 2) {
            resultsList.innerHTML = "";
            return;
        }
        fetchEntities(query);
    }, 300);
});

async function fetchEntities(query) {
    const response = await fetch(
        `/api/entities/search?q=${encodeURIComponent(query)}`,
    );
    const entities = await response.json();

    resultsList.innerHTML = "";
    for (const entity of entities) {
        const li = document.createElement("li");
        li.textContent = entity.display_name;
        li.dataset.entityId = entity.entity_id;
        li.addEventListener("click", () => selectEntity(li));
        resultsList.appendChild(li);
    }
}

function selectEntity(li) {
    document.querySelectorAll("#results li").forEach((el) => {
        el.classList.remove("selected");
    });
    li.classList.add("selected");
    entityIdInput.value = li.dataset.entityId;
    submitBtn.disabled = false;
}
