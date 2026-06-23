const MIN_QUERY_LENGTH = 2;
const DEBOUNCE_MS = 300;

const searchInput = document.querySelector("#search");
const resultsList = document.querySelector("#results");
const entityIdInput = document.querySelector("#entity_id");
const submitButton = document.querySelector("#submit-btn");

if (!(searchInput instanceof HTMLInputElement)) {
  throw new TypeError("#search must be an input element");
}

if (!(resultsList instanceof HTMLElement)) {
  throw new TypeError("#results must be a DOM element");
}

if (!(entityIdInput instanceof HTMLInputElement)) {
  throw new TypeError("#entity_id must be an input element");
}

if (!(submitButton instanceof HTMLButtonElement)) {
  throw new TypeError("#submit-btn must be a button element");
}

let debounceTimer = 0;

const selectEntity = (listItem) => {
  const resultItems = document.querySelectorAll("#results li");
  for (const resultItem of resultItems) {
    resultItem.classList.remove("selected");
  }

  listItem.classList.add("selected");
  entityIdInput.value = listItem.dataset.entityId ?? "";
  submitButton.disabled = false;
};

const renderEntities = (entities) => {
  resultsList.textContent = "";

  for (const entity of entities) {
    const listItem = document.createElement("li");
    listItem.textContent = entity.display_name;
    listItem.dataset.entityId = entity.entity_id;
    listItem.addEventListener("click", () => {
      selectEntity(listItem);
    });
    resultsList.append(listItem);
  }
};

const fetchEntities = (query) =>
  fetch(`/api/entities/search?q=${encodeURIComponent(query)}`)
    .then((response) => response.json())
    .then((entities) => {
      renderEntities(entities);
    });

const onSearchInput = () => {
  globalThis.clearTimeout(debounceTimer);
  debounceTimer = globalThis.setTimeout(() => {
    const query = searchInput.value.trim();
    if (query.length < MIN_QUERY_LENGTH) {
      resultsList.textContent = "";
      return;
    }

    fetchEntities(query);
  }, DEBOUNCE_MS);
};

searchInput.addEventListener("input", onSearchInput);
