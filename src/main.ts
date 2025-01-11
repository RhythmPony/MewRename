import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";

let rootInput: HTMLInputElement | null;
let browseButton: HTMLButtonElement | null;
let patternInput: HTMLInputElement | null;
let replacementInput: HTMLInputElement | null;
let useRegexCheckbox: HTMLInputElement | null;
let depthNumber: HTMLInputElement | null;
let targetSelect: HTMLSelectElement | null;
let countNumber: HTMLInputElement | null;
let filterInput: HTMLInputElement | null;
let resultArea: HTMLDivElement | null;
let resultTbody: HTMLTableSectionElement | null;

type PathInfo = [string, string, string, string];

async function createTable() {
  while (resultArea?.firstChild) {
    resultArea.removeChild(resultArea.firstChild);
  }

  let table = document.createElement("table");
  let colGroup = document.createElement("colgroup");
  for (let i = 0; i < 3; i++) {
    let col = document.createElement("col");
    colGroup.appendChild(col);
  }
  table.appendChild(colGroup);

  let thead = document.createElement("thead");
  let theadRow = document.createElement("tr");
  let selectCell = document.createElement("th");
  let sourceCell = document.createElement("th");
  let replacedCell = document.createElement("th");

  let selectBox = document.createElement("select");
  let selections = new Map([
    ["选择", "default"],
    ["全选", "all"],
    ["全不选", "none"],
    ["反选", "invert"],
  ]);
  for (let [text, value] of selections) {
    let option = document.createElement("option");
    option.textContent = text;
    option.value = value;
    selectBox.appendChild(option);
  }
  selectBox.addEventListener("change", (event) => {
    let option = (event.target as HTMLSelectElement).value;
    let checkboxes = table.querySelectorAll(
      'tbody input[type="checkbox"]'
    ) as NodeListOf<HTMLInputElement>;
    checkboxes.forEach((checkbox) => {
      if (option === "all") {
        checkbox.checked = true;
      } else if (option === "none") {
        checkbox.checked = false;
      } else if (option === "invert") {
        checkbox.checked = !checkbox.checked;
      }
    });
    let target = event.target as HTMLSelectElement;
    target.value = selections.values().next().value as string;
  });
  selectCell.appendChild(selectBox);

  sourceCell.textContent = "源字符串";
  replacedCell.textContent = "替换后字符串";

  theadRow.appendChild(selectCell);
  theadRow.appendChild(sourceCell);
  theadRow.appendChild(replacedCell);
  thead.appendChild(theadRow);
  table.appendChild(thead);

  let tbody = document.createElement("tbody");
  resultTbody = tbody;
  table.appendChild(tbody);
  resultArea?.appendChild(table);
}

async function addDataRows(data: [string, string, string, string][]) {
  data.forEach(([originPath, originName, targetPath, targetName]) => {
    let row = document.createElement("tr");
    let checkboxCell = document.createElement("td");
    let originCell = document.createElement("td");
    let targetCell = document.createElement("td");

    let checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = true;
    checkboxCell.appendChild(checkbox);

    originCell.innerHTML = originName;
    originCell.title = originPath;
    originCell.classList.add("t-cell");
    targetCell.innerHTML = targetName;
    targetCell.title = targetPath;
    targetCell.classList.add("t-cell");

    row.appendChild(checkboxCell);
    row.appendChild(originCell);
    row.appendChild(targetCell);
    resultTbody?.appendChild(row);
  });
}

async function foresights() {
  const root = rootInput?.value;
  const pattern = patternInput?.value || "";
  const replacement = replacementInput?.value || "";
  const useRegex = useRegexCheckbox?.checked;
  const depth = parseInt(depthNumber?.value as string) || 1;
  const target = targetSelect?.value || "NAME";
  const count = parseInt(countNumber?.value as string) || 0;
  const filter = filterInput?.value;

  try {
    let data = await invoke<[string, string, string, string][]>("foresights", {
      root: root,
      depth: depth,
      fileFilter: filter,
      pattern: pattern,
      replacement: replacement,
      useRegex: useRegex,
      target: target,
      count: count,
    });
    addDataRows(data);
  } catch (e) {
    console.error("Error:", e);
  }
}

function validatePattern() {
  if (!useRegexCheckbox?.checked) {
    patternInput?.setCustomValidity("");
    return;
  }
  let pattern = patternInput?.value;
  if (!pattern) {
    patternInput?.setCustomValidity("");
    return;
  }

  invoke<boolean>("validate_pattern", { pattern: pattern })
    .then((validity: boolean) => {
      if (validity) {
        patternInput?.setCustomValidity("");
      } else {
        patternInput?.setCustomValidity("无效的正则表达式");
      }
    })
    .catch((error) => {
      console.error("Error:", error);
    });
}

async function listenTauri() {
  await listen("foresights_event", (event) => {
    const data: PathInfo[] = event.payload as PathInfo[];
    addDataRows(data);
  });
}

window.addEventListener("DOMContentLoaded", () => {
  rootInput = document.querySelector("#root-input") as HTMLInputElement;
  browseButton = document.querySelector("#browse-button") as HTMLButtonElement;
  patternInput = document.querySelector("#pattern-input") as HTMLInputElement;
  replacementInput = document.querySelector(
    "#replacement-input"
  ) as HTMLInputElement;
  useRegexCheckbox = document.querySelector("#use-regex") as HTMLInputElement;
  depthNumber = document.querySelector("#depth-number") as HTMLInputElement;
  targetSelect = document.querySelector("target-select") as HTMLSelectElement;
  countNumber = document.querySelector("#count-number") as HTMLInputElement;
  filterInput = document.querySelector("#filter-input") as HTMLInputElement;
  resultArea = document.querySelector("#result-area") as HTMLDivElement;
  createTable();
  browseButton.addEventListener("click", async () => {
    const root = await open({
      multiple: false,
      directory: true,
    });
    if (root === null || rootInput === null) {
      return;
    }
    rootInput.value = root;
    foresights();
  });
  rootInput.addEventListener("blur", () => {
    foresights();
  });
  patternInput.addEventListener("blur", () => {
    validatePattern();
  });
  useRegexCheckbox.addEventListener("change", () => {
    validatePattern();
  });
  listenTauri();
});
