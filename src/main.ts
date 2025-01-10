import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import { getCurrentWindow } from '@tauri-apps/api/window';

// renames(paths: Vec<PathBuf>, pattern: String, replacement: String, use_regex: bool, target: String, count: isize, test:bool) 
// -> Result<Vec<PathBuf>, String>

// walk(root: PathBuf, depth: usize, file_only: bool)
// -> Result<Vec<PathBuf>, String>

// let rootInputEl: HTMLInputElement | null;
// let outputEl: HTMLInputElement | null;


// const walkDirectory = async () => {
//   try {
//       const paths: string[] = await invoke('walk_dir_wrapper', {
//           startPath: rootInputEl?.value,
//       });
//       for (const path of paths) {
//           outputEl?.insertAdjacentHTML('beforeend', `<li>${path}</li>`);
//       }
//   } catch (error) {
//       console.error('Error:', error);
//   }
// };

let rootInput: HTMLInputElement | null;
let browseButton: HTMLButtonElement | null;
// let patternInput: HTMLInputElement | null;
// let replacementInput: HTMLInputElement | null;
// let resultArea: HTMLDivElement | null;
let resultText: HTMLTextAreaElement | null;

function createTable(selector:string, data: [[string, string, string, string]]) {
  let container = document.querySelector(selector);
  if (!container) {
    return;
  }
  while (container.firstChild) {
    container.removeChild(container.firstChild);
  }

  let table = document.createElement('table');

  let thead = document.createElement('thead');
  let theadRow = document.createElement('tr');
  let selectCell = document.createElement('th');
  let sourceCell = document.createElement('th');
  let replacedCell = document.createElement('th');

  let selectBox = document.createElement('select');
  selectBox.innerHTML = `
    <option value="all">全选</option>
    <option value="none">全不选</option>
    <option value="invert">反选</option>
  `;
  selectBox.addEventListener('change', (event) => {
    let option = (event.target as HTMLSelectElement).value;
    let checkboxes = table.querySelectorAll('tbody input[type="checkbox"]') as NodeListOf<HTMLInputElement>;
    checkboxes.forEach((checkbox) => {
      if (option === 'all') {
        checkbox.checked = true;
      } else if (option === 'none') {
        checkbox.checked = false;
      } else if (option === 'invert') {
        checkbox.checked = !checkbox.checked;
      }
    });
  });
  selectCell.appendChild(selectBox);

  sourceCell.textContent = '源字符串';
  replacedCell.textContent = '替换后字符串';

  theadRow.appendChild(selectCell);
  theadRow.appendChild(sourceCell);
  theadRow.appendChild(replacedCell);
  thead.appendChild(theadRow);
  table.appendChild(thead);

  let tbody = document.createElement('tbody');
  data.forEach(([originPath, originName, targetPath, targetName]) => {
    let row = document.createElement('tr');
    let checkboxCell = document.createElement('td');
    let originCell = document.createElement('td');
    let targetCell = document.createElement('td');

    let checkbox = document.createElement('input');
    checkbox.type = 'checkbox';
    checkbox.checked = true;
    checkboxCell.appendChild(checkbox);

    originCell.innerHTML = originName;
    originCell.title = originPath;
    originCell.classList.add('t-cell');
    targetCell.innerHTML = targetName;
    targetCell.title = targetPath;
    targetCell.classList.add('t-cell');

    row.appendChild(checkboxCell);
    row.appendChild(originCell);
    row.appendChild(targetCell);
    tbody.appendChild(row);
  });
  table.appendChild(tbody);
  container.appendChild(table);
}

window.addEventListener("DOMContentLoaded", () => {
  const appWindow = getCurrentWindow();

  document
  .getElementById('titlebar-minimize')
  ?.addEventListener('click', () => appWindow.minimize());
  document
  .getElementById('titlebar-maximize')
  ?.addEventListener('click', () => appWindow.toggleMaximize());
  document
  .getElementById('titlebar-close')
  ?.addEventListener('click', () => appWindow.close());

  rootInput = document.querySelector("#root-input") as HTMLInputElement;
  browseButton = document.querySelector("#browse-button") as HTMLButtonElement;
  // resultArea = document.querySelector("#result-area") as HTMLDivElement;  
  resultText = document.querySelector("#result-text") as HTMLTextAreaElement;
  browseButton.addEventListener('click', async () => {
    const root = await open({
        multiple: false,
        directory: true,
    });
    if (root !== null && rootInput !== null) {
        rootInput.value = root;

    const depth = 2;
    const fileOnly = false;
    const count = -2;
    invoke<[[string,string,string,string]]>('foresights', {"root": root, "depth":depth, "fileOnly":fileOnly, "pattern": "\\wr", 
      "replacement": "FF<Aenum-4>", "useRegex": true, "target": "SUFFIX", "count": count})
      .then((fileTable) => {
        // if (resultText === null) {
        //   return;
        // }
        createTable('#result-area', fileTable);
      })
      .catch((error) => {
        console.error('Error:', error);
        if (resultText === null) {
          return;
        }
        resultText.textContent = error;
      });
    }
  });

  rootInput.addEventListener('change', () => {
      
  });
  // document.querySelector("#greet-form")?.addEventListener("submit", (e) => {
  //   e.preventDefault();
  //   walkDirectory();
  // });
});