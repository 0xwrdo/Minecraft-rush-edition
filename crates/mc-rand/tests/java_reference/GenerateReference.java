import java.io.FileWriter;
import java.io.PrintWriter;
import java.util.Random;

public class GenerateReference {
    public static void main(String[] args) throws Exception {
        String outputFile = args.length > 0 ? args[0] : "expected_random.csv";
        PrintWriter out = new PrintWriter(new FileWriter(outputFile));
        out.println("seed,op,index,value");

        long[] seeds = { 0L, 1L, 42L, 1337L, 8675309L, -1L, -987654321L };
        int[] bounds = { 16, 100, 64, 37, 1024, 999 };

        for (long seed : seeds) {
            Random r = new Random(seed);

            // 1. nextInt
            for (int i = 0; i < 10; i++) {
                out.printf("%d,nextInt,%d,%d\n", seed, i, r.nextInt());
            }

            // 2. nextInt(bound) - both power-of-two and rejection loop
            for (int i = 0; i < bounds.length; i++) {
                out.printf("%d,nextIntBound%d,%d,%d\n", seed, bounds[i], i, r.nextInt(bounds[i]));
            }

            // 3. nextLong
            for (int i = 0; i < 10; i++) {
                out.printf("%d,nextLong,%d,%d\n", seed, i, r.nextLong());
            }

            // 4. nextBoolean
            for (int i = 0; i < 10; i++) {
                out.printf("%d,nextBoolean,%d,%b\n", seed, i, r.nextBoolean());
            }

            // 5. nextFloat
            for (int i = 0; i < 10; i++) {
                out.printf("%d,nextFloat,%d,%.9f\n", seed, i, r.nextFloat());
            }

            // 6. nextDouble
            for (int i = 0; i < 10; i++) {
                out.printf("%d,nextDouble,%d,%.17f\n", seed, i, r.nextDouble());
            }

            // 7. nextGaussian
            for (int i = 0; i < 10; i++) {
                out.printf("%d,nextGaussian,%d,%.17f\n", seed, i, r.nextGaussian());
            }
        }

        out.close();
        System.out.println("Reference outputs written to: " + outputFile);
    }
}

