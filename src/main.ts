import { invoke } from "@tauri-apps/api/core";
import { open, confirm } from "@tauri-apps/plugin-dialog";
import { listen } from "@tauri-apps/api/event";
import { WebviewWindow } from "@tauri-apps/api/webviewWindow";
import { disableRefresh } from "./uiconfig";

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
let executeButton: HTMLButtonElement | null;
let pending = false;

type PathInfo = [string, string, string, string];

async function initTable() {
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
  data.forEach(([originalPath, originalName, targetPath, targetName]) => {
    let row = document.createElement("tr");
    let checkboxCell = document.createElement("td");
    let originalCell = document.createElement("td");
    let targetCell = document.createElement("td");

    let checkbox = document.createElement("input");
    checkbox.type = "checkbox";
    checkbox.checked = true;
    checkboxCell.appendChild(checkbox);

    originalCell.innerHTML = originalName;
    originalCell.title = originalPath;
    originalCell.dataset.path = originalPath;
    originalCell.classList.add("t-cell");
    targetCell.innerHTML = targetName;
    targetCell.title = targetPath;
    targetCell.dataset.path = targetPath;
    targetCell.classList.add("t-cell");

    row.appendChild(checkboxCell);
    row.appendChild(originalCell);
    row.appendChild(targetCell);
    resultTbody?.appendChild(row);
  });
}

let getParams = () => {
  const root = rootInput?.value;
  const pattern = patternInput?.value || "";
  const replacement = replacementInput?.value || "";
  const useRegex = useRegexCheckbox?.checked;
  const depth =
    parseInt(depthNumber?.value as string) === 0
      ? 0
      : parseInt(depthNumber?.value as string) || 1;
  const target = targetSelect?.value || "NAME";
  const count = parseInt(countNumber?.value as string) || 0;
  const fileFilter = filterInput?.value;
  return {
    root,
    pattern,
    replacement,
    useRegex,
    depth,
    target,
    count,
    fileFilter,
  };
};

async function foresights() {
  try {
    if (pending) {
      return;
    }
    initTable();
    pending = true;
    let params = getParams();
    await invoke<[string, string, string, string][]>("foresights", params);
  } catch (e) {
    console.error("Error:", e);
  }
}

async function updateForesights() {
  try {
    let table = document.querySelector("#table-container table");
    let rows = table?.querySelectorAll("tr");
    if (!rows) {
      return;
    }
    const pattern = patternInput?.value || "";
    const replacement = replacementInput?.value || "";
    const useRegex = useRegexCheckbox?.checked;
    const target = targetSelect?.value || "NAME";
    const count = parseInt(countNumber?.value as string) || 0;
    let serialNumber = 1;

    pending = true;
    for (let row of rows) {
      let cell = row.children[1] as HTMLTableCellElement;
      let path = cell.dataset.path;
      if (!path || path === null) {
        continue;
      }
      const [originalPath, originalName, targetPath, targetName] = await invoke<
        [string, string, string, string]
      >("foresight_with_serial", {
        path,
        pattern,
        replacement,
        useRegex,
        target,
        count,
        serialNumber,
      });
      if (originalPath !== targetPath) {
        serialNumber += 1;
      }
      const originalCell = row.children[1] as HTMLTableCellElement;
      originalCell.innerHTML = originalName;
      originalCell.title = originalPath;
      originalCell.dataset.path = originalPath;

      const targetCell = row.children[2] as HTMLTableCellElement;
      targetCell.innerHTML = targetName;
      targetCell.title = targetPath;
      targetCell.dataset.path = targetPath;
    }
    pending = false;
  } catch (e) {
    console.error("Error:", e);
  }
}

async function renames() {
  try {
    let table = document.querySelector("#table-container table");
    let rows = table?.querySelectorAll("tr");
    if (!rows) {
      return;
    }

    const reversedRows = Array.from(rows).reverse();

    pending = true;
    for (let row of reversedRows) {
      const checkCell = row.children[0].querySelector(
        "input[type=checkbox]"
      ) as HTMLInputElement;
      if (!checkCell?.checked) {
        continue;
      }
      const originalCell = row.children[1] as HTMLTableCellElement;
      let originalPath = originalCell.dataset.path;

      const targetCell = row.children[2] as HTMLTableCellElement;
      let targetPath = targetCell.dataset.path;
      if (
        !originalPath ||
        originalPath === null ||
        !targetPath ||
        targetPath === null
      ) {
        continue;
      }
      if (originalPath === targetPath) {
        continue;
      }
      const result = await invoke<[string, string, string, string]>("rename", {
        originalPath,
        targetPath,
      });
      console.log(result);
    }
    pending = false;
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

async function listenForesights() {
  await listen("foresights_event", (event) => {
    const data: PathInfo[] = event.payload as PathInfo[];
    if (data === null) {
      pending = false;
      return;
    }
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
  targetSelect = document.querySelector("#target-select") as HTMLSelectElement;
  countNumber = document.querySelector("#count-number") as HTMLInputElement;
  filterInput = document.querySelector("#filter-input") as HTMLInputElement;
  resultArea = document.querySelector("#table-container") as HTMLDivElement;
  executeButton = document.querySelector(
    "#execute-button"
  ) as HTMLButtonElement;
  initTable();
  browseButton.addEventListener("click", async (e) => {
    e.preventDefault();
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
  rootInput.addEventListener("blur", (e) => {
    e.preventDefault();
    foresights();
  });
  depthNumber.addEventListener("change", (e) => {
    e.preventDefault();
    foresights();
  });
  filterInput.addEventListener("change", (e) => {
    e.preventDefault();
    foresights();
  });
  patternInput.addEventListener("blur", (e) => {
    e.preventDefault();
    validatePattern();
    updateForesights();
  });
  replacementInput.addEventListener("blur", (e) => {
    e.preventDefault();
    updateForesights();
  });
  useRegexCheckbox.addEventListener("change", (e) => {
    e.preventDefault();
    validatePattern();
    updateForesights();
  });
  countNumber.addEventListener("change", (e) => {
    e.preventDefault();
    updateForesights();
  });
  targetSelect.addEventListener("change", (e) => {
    e.preventDefault();
    updateForesights();
  });
  executeButton.addEventListener("click", async (e) => {
    e.preventDefault();
    const confirmation = await confirm("此操作不可逆。确认继续？", {
      title: "警告",
      kind: "warning",
    });
    if (confirmation) {
      renames();
    }
  });
  listenForesights();

  document.querySelector("#nav-about")?.addEventListener("click", async () => {
    const webview = new WebviewWindow("about-view", {
      title: "关于 Mew Rename",
      url: "./src/subwindows/about.html",
      width: 400,
      height: 300,
      resizable: false,
      maximizable: false,
      minimizable: false,
      center: true,
    });
    webview.once("tauri://created", function () {
      // webview successfully created
    });
    webview.once("tauri://error", function (e) {
      console.log(e);
      // an error happened creating the webview
    });
  });

  document.querySelector("#nav-help")?.addEventListener("click", async () => {
    const webview = new WebviewWindow("help-view", {
      title: "帮助",
      url: "./src/subwindows/help.html",
      width: 600,
      height: 450,
      resizable: false,
      maximizable: false,
      minimizable: false,
      center: true,
    });
    webview.once("tauri://created", function () {
      // webview successfully created
    });
    webview.once("tauri://error", function (e) {
      console.log(e);
      // an error happened creating the webview
    });
  });

  document.querySelector("#nav-donate")?.addEventListener("click", async () => {
    const webview = new WebviewWindow("donate-view", {
      title: "捐赠",
      url: "./src/subwindows/donate.html",
      width: 450,
      height: 600,
      resizable: false,
      maximizable: false,
      minimizable: false,
      center: true,
    });
    webview.once("tauri://created", function () {
      // webview successfully created
    });
    webview.once("tauri://error", function (e) {
      console.log(e);
      // an error happened creating the webview
    });
  });

  document
    .querySelector("#nav-licence")
    ?.addEventListener("click", async () => {
      const webview = new WebviewWindow("licence-view", {
        title: "许可证",
        url: "./src/subwindows/licence.html",
        width: 600,
        height: 450,
        resizable: false,
        maximizable: false,
        minimizable: false,
        center: true,
      });
      webview.once("tauri://created", function () {
        // webview successfully created
      });
      webview.once("tauri://error", function (e) {
        console.log(e);
        // an error happened creating the webview
      });
    });

  disableRefresh();
});
