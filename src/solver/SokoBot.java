package solver;

import java.io.*;
import java.util.ArrayList;
import java.util.List;

public class SokoBot {
    
    public String solveSokobanPuzzle(int width, int height, char[][] mapData, char[][] itemsData) {
        try {
            File tempInput = File.createTempFile("sokoban_puzzle", ".txt");
            tempInput.deleteOnExit();
            
            writePuzzleToFile(tempInput, width, height, mapData, itemsData);
            String solution = callRustSolver(tempInput.getAbsolutePath());
            
            return solution != null ? solution : "";
            
        } catch (Exception e) {
            e.printStackTrace();
            return "";
        }
    }
    
    private void writePuzzleToFile(File file, int width, int height, char[][] mapData, char[][] itemsData) throws IOException {
        try (BufferedWriter writer = new BufferedWriter(new FileWriter(file))) {
            for (int y = 0; y < height; y++) {
                StringBuilder line = new StringBuilder();
                for (int x = 0; x < width; x++) {
                    char tile = mapData[y][x];
                    char item = itemsData[y][x];
                    
                    if (item == '@') {
                        line.append(tile == '.' ? '+' : '@');
                    } else if (item == '$') {
                        line.append(tile == '.' ? '*' : '$');
                    } else if (tile == '.') {
                        line.append('.');
                    } else if (tile == '#') {
                        line.append('#');
                    } else {
                        line.append(' ');
                    }
                }
                writer.write(line.toString());
                writer.newLine();
            }
        }
    }
    
    private String callRustSolver(String inputFilePath) throws IOException, InterruptedException {
        List<String> command = new ArrayList<>();
        command.add("./rust_solver");
        command.add(inputFilePath);
        
        ProcessBuilder processBuilder = new ProcessBuilder(command);
        processBuilder.redirectErrorStream(true);
        
        Process process = processBuilder.start();
        
        StringBuilder solution = new StringBuilder();
        try (BufferedReader reader = new BufferedReader(new InputStreamReader(process.getInputStream()))) {
            String line;
            while ((line = reader.readLine()) != null) {
                solution.append(line);
            }
        }
        
        boolean completed = process.waitFor(30, java.util.concurrent.TimeUnit.SECONDS);
        
        if (!completed) {
            process.destroyForcibly();
            System.err.println("Rust solver timed out");
            return "";
        }
        
        int exitCode = process.exitValue();
        
        if (exitCode == 0) {
            return solution.toString().trim();
        } else {
            System.err.println("Rust solver exited with code: " + exitCode);
            return "";
        }
    }
}
